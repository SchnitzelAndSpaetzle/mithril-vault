// SPDX-License-Identifier: MIT

//! File locking service for preventing concurrent database access.
//!
//! This module implements a hybrid locking mechanism:
//! 1. OS-level advisory locks via `fs4` for cross-platform file locking
//! 2. Lock files (`.kdbx.lock`) for metadata and stale lock detection
//!
//! The lock file contains JSON metadata including PID, hostname, and timestamp,
//! allowing detection of stale locks from crashed processes.

use chrono::{DateTime, Utc};
use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use sysinfo::{Pid, System};

use crate::dto::error::AppError;

/// Information stored in the lock file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFileInfo {
    /// Process ID that holds the lock
    pub pid: u32,
    /// Application name
    pub application: String,
    /// Application version
    pub version: String,
    /// When the lock was acquired
    pub opened_at: DateTime<Utc>,
    /// Hostname of the machine holding the lock
    pub hostname: String,
}

impl LockFileInfo {
    /// Creates lock file info for the current process.
    fn for_current_process() -> Self {
        Self {
            pid: process::id(),
            application: "MithrilVault".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            opened_at: Utc::now(),
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}

/// Status of a database lock.
#[derive(Debug, Clone)]
pub enum LockStatus {
    /// No lock exists, a database can be opened
    Available,
    /// Lock is held by the current process
    LockedByCurrentProcess,
    /// Lock is held by another running process
    LockedByOtherProcess(LockFileInfo),
    /// Lock file exists, but the process is no longer running
    StaleLock(LockFileInfo),
}

/// Represents an acquired file lock.
///
/// The lock is automatically released when this struct is dropped.
#[derive(Debug)]
pub struct FileLock {
    /// The database file handle with OS-level lock
    db_file: File,
    /// Path to the lock metadata file
    lock_file_path: PathBuf,
}

impl FileLock {
    /// Returns the path to the lock file.
    pub fn lock_file_path(&self) -> &Path {
        &self.lock_file_path
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // Release OS-level lock
        let _ = self.db_file.unlock();

        // Remove lock metadata file
        let _ = std::fs::remove_file(&self.lock_file_path);
    }
}

/// Service for managing database file locks.
pub struct FileLockService;

impl FileLockService {
    /// Returns the path to the lock file for a given database path.
    pub fn lock_file_path(db_path: &str) -> PathBuf {
        let mut path = PathBuf::from(db_path);
        let file_name = path
            .file_name()
            .map(|n| format!("{}.lock", n.to_string_lossy()))
            .unwrap_or_else(|| "database.kdbx.lock".to_string());
        path.set_file_name(file_name);
        path
    }

    /// Attempts to acquire a lock on the database file.
    ///
    /// This method:
    /// 1. Checks for existing lock files and validates them
    /// 2. Acquires an OS-level exclusive lock on the database file
    /// 3. Creates a lock metadata file with process information
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database file doesn't exist or can't be opened
    /// - Another process holds an active lock
    /// - File system operations fail
    pub fn try_acquire_lock(db_path: &str) -> Result<FileLock, AppError> {
        let lock_file_path = Self::lock_file_path(db_path);

        // Check for existing lock
        match Self::check_lock_status(db_path)? {
            LockStatus::Available => {}
            LockStatus::LockedByCurrentProcess => {
                return Err(AppError::DatabaseAlreadyOpen);
            }
            LockStatus::LockedByOtherProcess(info) => {
                return Err(AppError::DatabaseLocked(format!(
                    "Database is locked by {} (PID: {}) on {} since {}",
                    info.application,
                    info.pid,
                    info.hostname,
                    info.opened_at.format("%Y-%m-%d %H:%M:%S UTC")
                )));
            }
            LockStatus::StaleLock(_info) => {
                // Clean up stale lock and continue
                Self::remove_lock_file(&lock_file_path)?;
            }
        }

        // Open a database file for locking (read mode is enough for advisory lock)
        let db_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(db_path)
            .map_err(|e| AppError::InvalidPath(format!("Cannot open database file: {e}")))?;

        // Try to acquire OS-level exclusive lock (non-blocking)
        // fs4's try_lock_exclusive returns Ok(()) on success, Err with WouldBlock if locked
        if let Err(e) = db_file.try_lock_exclusive() {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                return Err(AppError::FileLockFailed(
                    "Database file is locked by another process".to_string(),
                ));
            }
            return Err(AppError::FileLockFailed(format!(
                "Failed to acquire file lock: {e}"
            )));
        }

        // Create a lock metadata file
        let lock_info = LockFileInfo::for_current_process();
        Self::write_lock_file(&lock_file_path, &lock_info)?;

        Ok(FileLock {
            db_file,
            lock_file_path,
        })
    }

    /// Checks the lock status for a database without acquiring a lock.
    ///
    /// This is useful for:
    /// - Displaying lock information in the UI
    /// - Determining if force unlock is needed
    pub fn check_lock_status(db_path: &str) -> Result<LockStatus, AppError> {
        let lock_file_path = Self::lock_file_path(db_path);

        // If no lock file exists, the database is available
        if !lock_file_path.exists() {
            return Ok(LockStatus::Available);
        }

        // Read and parse lock file
        let lock_info = match Self::read_lock_file(&lock_file_path) {
            Ok(info) => info,
            Err(_) => {
                // Lock file exists but is corrupted - treat as stale
                return Ok(LockStatus::StaleLock(LockFileInfo {
                    pid: 0,
                    application: "Unknown".to_string(),
                    version: "Unknown".to_string(),
                    opened_at: Utc::now(),
                    hostname: "Unknown".to_string(),
                }));
            }
        };

        // Check if it's our own process
        if lock_info.pid == process::id() {
            return Ok(LockStatus::LockedByCurrentProcess);
        }

        // Check if the process is still running
        if Self::is_process_running(lock_info.pid) {
            Ok(LockStatus::LockedByOtherProcess(lock_info))
        } else {
            Ok(LockStatus::StaleLock(lock_info))
        }
    }

    /// Forces removal of a lock file.
    ///
    /// This should only be used for recovery when:
    /// - The user confirms they want to force unlock
    /// - The lock is known to be stale
    ///
    /// # Warning
    ///
    /// Using this on an actively locked database may cause data corruption.
    pub fn force_unlock(db_path: &str) -> Result<(), AppError> {
        let lock_file_path = Self::lock_file_path(db_path);

        if lock_file_path.exists() {
            Self::remove_lock_file(&lock_file_path)?;
        }

        Ok(())
    }

    /// Checks if a process with the given PID is currently running.
    fn is_process_running(pid: u32) -> bool {
        let mut system = System::new();
        system.refresh_processes(
            sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
            true,
        );
        system.process(Pid::from_u32(pid)).is_some()
    }

    /// Reads and parses a lock file.
    fn read_lock_file(path: &Path) -> Result<LockFileInfo, AppError> {
        let mut file = File::open(path).map_err(|e| AppError::Io(e.to_string()))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| AppError::Io(e.to_string()))?;

        serde_json::from_str(&contents)
            .map_err(|e| AppError::Io(format!("Invalid lock file format: {e}")))
    }

    /// Writes lock information to a lock file.
    fn write_lock_file(path: &Path, info: &LockFileInfo) -> Result<(), AppError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| AppError::Io(format!("Cannot create lock file: {e}")))?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, permissions)
                .map_err(|e| AppError::Io(format!("Cannot set lock file permissions: {e}")))?;
        }

        let json = serde_json::to_string_pretty(info)
            .map_err(|e| AppError::Io(format!("Cannot serialize lock info: {e}")))?;

        file.write_all(json.as_bytes())
            .map_err(|e| AppError::Io(format!("Cannot write lock file: {e}")))?;

        file.sync_all()
            .map_err(|e| AppError::Io(format!("Cannot sync lock file: {e}")))?;

        Ok(())
    }

    /// Removes a lock file.
    fn remove_lock_file(path: &Path) -> Result<(), AppError> {
        std::fs::remove_file(path)
            .map_err(|e| AppError::Io(format!("Cannot remove lock file: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_db(dir: &TempDir) -> PathBuf {
        let db_path = dir.path().join("test.kdbx");
        let mut file = File::create(&db_path).unwrap();
        file.write_all(b"test database content").unwrap();
        db_path
    }

    #[test]
    fn test_lock_file_path() {
        let path = FileLockService::lock_file_path("/path/to/vault.kdbx");
        assert_eq!(path, PathBuf::from("/path/to/vault.kdbx.lock"));
    }

    #[test]
    fn test_lock_file_path_no_extension() {
        let path = FileLockService::lock_file_path("/path/to/vault");
        assert_eq!(path, PathBuf::from("/path/to/vault.lock"));
    }

    #[test]
    fn test_lock_status_available_when_no_lock_file() {
        let dir = TempDir::new().unwrap();
        let db_path = create_test_db(&dir);

        let status = FileLockService::check_lock_status(db_path.to_str().unwrap()).unwrap();
        assert!(matches!(status, LockStatus::Available));
    }

    #[test]
    fn test_acquire_and_release_lock() {
        let dir = TempDir::new().unwrap();
        let db_path = create_test_db(&dir);
        let db_path_str = db_path.to_str().unwrap();
        let lock_file_path = FileLockService::lock_file_path(db_path_str);

        // Acquire lock
        let lock = FileLockService::try_acquire_lock(db_path_str).unwrap();

        // Lock file should exist
        assert!(lock_file_path.exists());

        // Status should show locked by current process
        let status = FileLockService::check_lock_status(db_path_str).unwrap();
        assert!(matches!(status, LockStatus::LockedByCurrentProcess));

        // Drop lock
        drop(lock);

        // Lock file should be removed
        assert!(!lock_file_path.exists());

        // Status should show available
        let status = FileLockService::check_lock_status(db_path_str).unwrap();
        assert!(matches!(status, LockStatus::Available));
    }

    #[test]
    fn test_cannot_acquire_lock_twice() {
        let dir = TempDir::new().unwrap();
        let db_path = create_test_db(&dir);
        let db_path_str = db_path.to_str().unwrap();

        // Acquire first lock
        let _lock = FileLockService::try_acquire_lock(db_path_str).unwrap();

        // Try to acquire second lock - should fail
        let result = FileLockService::try_acquire_lock(db_path_str);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::DatabaseAlreadyOpen));
    }

    #[test]
    fn test_stale_lock_detection() {
        let dir = TempDir::new().unwrap();
        let db_path = create_test_db(&dir);
        let db_path_str = db_path.to_str().unwrap();
        let lock_file_path = FileLockService::lock_file_path(db_path_str);

        // Create a fake lock file with a non-existent PID
        let fake_info = LockFileInfo {
            pid: 999_999_999, // Very unlikely to exist
            application: "OtherApp".to_string(),
            version: "1.0.0".to_string(),
            opened_at: Utc::now(),
            hostname: "other-host".to_string(),
        };
        FileLockService::write_lock_file(&lock_file_path, &fake_info).unwrap();

        // Status should show stale lock
        let status = FileLockService::check_lock_status(db_path_str).unwrap();
        assert!(matches!(status, LockStatus::StaleLock(_)));

        // Should be able to acquire lock (stale lock is cleaned up)
        let lock = FileLockService::try_acquire_lock(db_path_str).unwrap();
        assert!(lock_file_path.exists());
        drop(lock);
    }

    #[test]
    fn test_force_unlock() {
        let dir = TempDir::new().unwrap();
        let db_path = create_test_db(&dir);
        let db_path_str = db_path.to_str().unwrap();
        let lock_file_path = FileLockService::lock_file_path(db_path_str);

        // Create a lock file
        let fake_info = LockFileInfo::for_current_process();
        FileLockService::write_lock_file(&lock_file_path, &fake_info).unwrap();
        assert!(lock_file_path.exists());

        // Force unlock
        FileLockService::force_unlock(db_path_str).unwrap();

        // Lock file should be removed
        assert!(!lock_file_path.exists());
    }

    #[test]
    fn test_force_unlock_nonexistent_is_ok() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("nonexistent.kdbx");

        // Force unlock on nonexistent lock file should succeed
        let result = FileLockService::force_unlock(db_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_file_info_serialization() {
        let info = LockFileInfo::for_current_process();
        let json = serde_json::to_string(&info).unwrap();
        let parsed: LockFileInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info.pid, parsed.pid);
        assert_eq!(info.application, parsed.application);
        assert_eq!(info.hostname, parsed.hostname);
    }

    #[test]
    fn test_corrupted_lock_file_treated_as_stale() {
        let dir = TempDir::new().unwrap();
        let db_path = create_test_db(&dir);
        let db_path_str = db_path.to_str().unwrap();
        let lock_file_path = FileLockService::lock_file_path(db_path_str);

        // Create a corrupted lock file
        let mut file = File::create(&lock_file_path).unwrap();
        file.write_all(b"not valid json").unwrap();

        // Status should show stale lock (corrupted is treated as stale)
        let status = FileLockService::check_lock_status(db_path_str).unwrap();
        assert!(matches!(status, LockStatus::StaleLock(_)));
    }

    #[test]
    fn test_is_process_running_current_process() {
        // Current process should always be running
        assert!(FileLockService::is_process_running(process::id()));
    }

    #[test]
    fn test_is_process_running_nonexistent() {
        // Very high PID should not exist
        assert!(!FileLockService::is_process_running(999_999_999));
    }
}
