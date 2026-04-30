pub mod create;
pub mod entries;
pub mod favicons;
pub mod groups;
pub mod header;
pub mod key;
pub mod keyfile;
pub mod mapping;
pub mod open;
pub mod save;

use crate::domain::kdbx::OpenDatabase;
use crate::dto::database::DatabaseInfo;
use crate::dto::error::AppError;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub struct KdbxService {
    /// Map of normalized database paths to open databases.
    /// The key is the canonical/normalized path to ensure consistent lookups.
    databases: Mutex<HashMap<String, OpenDatabase>>,
    /// Domain-level cooldown map for failed favicon fetches.
    favicon_failed_domains: Mutex<HashMap<String, Instant>>,
}

impl KdbxService {
    pub const FAVICON_FAILURE_COOLDOWN: Duration = Duration::from_secs(15 * 60);

    /// Creates a new KDBX service.
    pub fn new() -> Self {
        Self {
            databases: Mutex::new(HashMap::new()),
            favicon_failed_domains: Mutex::new(HashMap::new()),
        }
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

    pub(crate) fn is_favicon_domain_on_cooldown(&self, domain: &str) -> Result<bool, AppError> {
        let mut failures = self
            .favicon_failed_domains
            .lock()
            .map_err(|_| AppError::Lock)?;
        let Some(last_failed_at) = failures.get(domain) else {
            return Ok(false);
        };
        if last_failed_at.elapsed() >= Self::FAVICON_FAILURE_COOLDOWN {
            failures.remove(domain);
            return Ok(false);
        }
        Ok(true)
    }

    pub(crate) fn mark_favicon_domain_failed(&self, domain: &str) -> Result<(), AppError> {
        let mut failures = self
            .favicon_failed_domains
            .lock()
            .map_err(|_| AppError::Lock)?;
        failures.insert(domain.to_string(), Instant::now());
        Ok(())
    }

    pub(crate) fn clear_favicon_domain_failure(&self, domain: &str) -> Result<(), AppError> {
        let mut failures = self
            .favicon_failed_domains
            .lock()
            .map_err(|_| AppError::Lock)?;
        failures.remove(domain);
        Ok(())
    }

    /// Checks if a database at the given path is already open.
    pub fn is_database_open(&self, path: &str) -> Result<bool, AppError> {
        let normalized = Self::normalize_path(path);
        let databases = self.lock_databases()?;
        Ok(databases.contains_key(&normalized))
    }

    /// Returns a list of all currently open databases.
    pub fn list_open_databases(&self) -> Result<Vec<DatabaseInfo>, AppError> {
        let databases = self.lock_databases()?;
        let mut infos = Vec::with_capacity(databases.len());

        for open_db in databases.values() {
            infos.push(DatabaseInfo {
                name: open_db.db.root.name.clone(),
                path: open_db.path.clone(),
                is_modified: open_db.is_modified,
                is_locked: false,
                root_group_id: open_db.db.root.uuid.to_string(),
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
