// SPDX-License-Identifier: MIT

//! DTOs for database file locking.

use serde::{Deserialize, Serialize};

/// Information about who holds a lock on a database file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockFileInfoDto {
    /// Process ID that holds the lock
    pub pid: u32,
    /// Application name
    pub application: String,
    /// Application version
    pub version: String,
    /// ISO 8601 timestamp when the lock was acquired
    pub opened_at: String,
    /// Hostname of the machine holding the lock
    pub hostname: String,
}

/// Status of a database file lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum LockStatusDto {
    /// No lock exists, database can be opened
    Available,
    /// Lock is held by the current process
    LockedByCurrentProcess,
    /// Lock is held by another running process
    #[serde(rename_all = "camelCase")]
    LockedByOtherProcess {
        /// Information about the process holding the lock
        info: LockFileInfoDto,
    },
    /// Lock file exists but the process is no longer running
    #[serde(rename_all = "camelCase")]
    StaleLock {
        /// Information about the stale lock
        info: LockFileInfoDto,
    },
}

impl From<crate::services::file_lock::LockFileInfo> for LockFileInfoDto {
    fn from(info: crate::services::file_lock::LockFileInfo) -> Self {
        Self {
            pid: info.pid,
            application: info.application,
            version: info.version,
            opened_at: info.opened_at.to_rfc3339(),
            hostname: info.hostname,
        }
    }
}

impl From<crate::services::file_lock::LockStatus> for LockStatusDto {
    fn from(status: crate::services::file_lock::LockStatus) -> Self {
        match status {
            crate::services::file_lock::LockStatus::Available => Self::Available,
            crate::services::file_lock::LockStatus::LockedByCurrentProcess => {
                Self::LockedByCurrentProcess
            }
            crate::services::file_lock::LockStatus::LockedByOtherProcess(info) => {
                Self::LockedByOtherProcess { info: info.into() }
            }
            crate::services::file_lock::LockStatus::StaleLock(info) => {
                Self::StaleLock { info: info.into() }
            }
        }
    }
}
