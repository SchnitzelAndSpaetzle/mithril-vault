// SPDX-License-Identifier: MIT

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database not open")]
    DatabaseNotOpen,

    #[error("Database already open: {0}")]
    DatabaseAlreadyOpen(String),

    #[error("Database not found: {0}")]
    DatabaseNotFound(String),

    #[error("Database is locked: {0}")]
    DatabaseLocked(String),

    #[error("Database has unsaved changes: {0}")]
    DatabaseModified(String),

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Entry not found: {0}")]
    EntryNotFound(String),

    #[error("Custom field not found: {0}")]
    CustomFieldNotFound(String),

    #[error("Attachment not found: {0}")]
    AttachmentNotFound(String),

    #[error("Attachment '{filename}' is {size} bytes, exceeding the {cap}-byte limit")]
    AttachmentTooLarge {
        filename: String,
        size: u64,
        cap: u64,
    },

    #[error("Custom field is not protected: {0}")]
    CustomFieldNotProtected(String),

    #[error("History version no longer matches: index {0} has changed or was pruned")]
    HistoryVersionMismatch(usize),

    #[error("History version unchanged: this version's content matches the current entry")]
    HistoryVersionUnchanged,

    #[error("Group not found: {0}")]
    GroupNotFound(String),

    #[error("Cannot delete root group")]
    CannotDeleteRootGroup,

    #[error("Cannot move root group")]
    CannotMoveRootGroup,

    #[error("Cannot move group into itself or its descendants")]
    CircularReference,

    #[error("Group is not empty and recursive delete not requested")]
    GroupNotEmpty(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("KDBX error: {0}")]
    Kdbx(String),

    #[error("Not a valid KDBX file")]
    InvalidKdbxFile,

    #[error("Unsupported KDBX version: {0}")]
    UnsupportedKdbxVersion(String),

    #[error("Header integrity check failed")]
    HeaderIntegrityError,

    #[error("Unsupported cipher: {0}")]
    UnsupportedCipher(String),

    #[error("Unsupported KDF: {0}")]
    UnsupportedKdf(String),

    #[error("Header parse error: {0}")]
    HeaderParseError(String),

    #[error("Keyfile not found")]
    KeyfileNotFound,

    #[error("Invalid keyfile format")]
    KeyfileInvalid,

    #[error("No credentials provided")]
    NoCredentials,

    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Secure storage error: {0}")]
    SecureStorage(String),

    #[error("Lock error")]
    Lock,

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Atomic write failed: {0}")]
    AtomicWrite(String),

    #[error("Failed to sync file to disk: {0}")]
    SyncFailed(String),

    #[error("Window protection error: {0}")]
    WindowProtection(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Backup failed for {path}: {reason}")]
    BackupFailed { path: String, reason: String },

    #[error("Audit log read failed: {0}")]
    AuditRead(String),

    #[error("Audit log clear failed: {0}")]
    AuditClear(String),

    #[error("The selected file is not a copy of the open Vault")]
    MergeUnrelatedVault,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<crate::services::kdbx::backups::BackupError> for AppError {
    fn from(err: crate::services::kdbx::backups::BackupError) -> Self {
        match err {
            crate::services::kdbx::backups::BackupError::BackupFailed { path, source } => {
                Self::BackupFailed {
                    path: path.to_string_lossy().into_owned(),
                    reason: source.to_string(),
                }
            }
        }
    }
}
