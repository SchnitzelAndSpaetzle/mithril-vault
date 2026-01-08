// SPDX-License-Identifier: GPL-3.0-or-later

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

    #[error("Group not found: {0}")]
    GroupNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("KDBX error: {0}")]
    Kdbx(String),

    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Lock error")]
    Lock,

    #[error("Not implemented: {0}")]
    NotImplemented(String),
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
