// SPDX-License-Identifier: MIT

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database not open")]
    DatabaseNotOpen,

    #[error("Database already open")]
    DatabaseAlreadyOpen,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Entry not found: {0}")]
    EntryNotFound(String),

    #[error("Custom field not found: {0}")]
    CustomFieldNotFound(String),

    #[error("Custom field is not protected: {0}")]
    CustomFieldNotProtected(String),

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
