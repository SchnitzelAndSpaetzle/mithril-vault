// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    pub name: String,
    pub path: String,
    pub is_modified: bool,
    pub is_locked: bool,
    pub root_group_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStats {
    pub entry_count: usize,
    pub group_count: usize,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
}

/// Options for creating a new KDBX4 database
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseCreationOptions {
    /// Database description (optional metadata)
    pub description: Option<String>,
    /// Whether to create default groups (General, Email, Banking, Social)
    #[serde(default)]
    pub create_default_groups: bool,
    /// Argon2id memory parameter in bytes (default: 64 MB)
    pub kdf_memory: Option<u64>,
    /// Argon2id iterations/time cost (default: 3)
    pub kdf_iterations: Option<u64>,
    /// Argon2id parallelism/lanes (default: 4)
    pub kdf_parallelism: Option<u32>,
}

impl DatabaseCreationOptions {
    /// Get KDF memory in bytes (default: 64 MB)
    pub fn memory_bytes(&self) -> u64 {
        self.kdf_memory.unwrap_or(64 * 1024 * 1024)
    }

    /// Get KDF iterations (default: 3)
    pub fn iterations(&self) -> u64 {
        self.kdf_iterations.unwrap_or(3)
    }

    /// Get KDF parallelism (default: 4)
    pub fn parallelism(&self) -> u32 {
        self.kdf_parallelism.unwrap_or(4)
    }
}

/// Pre-authentication database inspection result.
/// Contains information that can be read from KDBX headers without needing credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHeaderInfo {
    /// Database format version (e.g., "KDBX 4.0", "KDBX 3.1")
    pub version: String,
    /// Whether the file has valid KDBX magic bytes
    pub is_valid_kdbx: bool,
    /// Whether the KDBX version is supported by this application
    pub is_supported: bool,
    /// Path to the database file
    pub path: String,
}

/// Outer encryption cipher algorithms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OuterCipher {
    /// AES-256 in CBC mode
    Aes256,
    /// Twofish cipher
    Twofish,
    /// `ChaCha20` stream cipher
    ChaCha20,
}

/// Inner stream cipher algorithms for protecting in-memory values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InnerCipher {
    /// No encryption (plaintext)
    Plain,
    /// Salsa20 stream cipher
    Salsa20,
    /// `ChaCha20` stream cipher
    ChaCha20,
}

/// Compression algorithms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Compression {
    /// No compression
    None,
    /// `GZip` compression
    GZip,
}

/// Key derivation function (KDF) settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum KdfSettings {
    /// AES-based KDF (KDBX 3.x)
    #[serde(rename_all = "camelCase")]
    AesKdf {
        /// Number of transformation rounds
        rounds: u64,
    },
    /// Argon2d KDF (KDBX 4.x)
    #[serde(rename_all = "camelCase")]
    Argon2d {
        /// Memory usage in bytes
        memory: u64,
        /// Number of iterations (time cost)
        iterations: u64,
        /// Degree of parallelism (lanes)
        parallelism: u32,
    },
    /// Argon2id KDF (KDBX 4.x, recommended)
    #[serde(rename_all = "camelCase")]
    Argon2id {
        /// Memory usage in bytes
        memory: u64,
        /// Number of iterations (time cost)
        iterations: u64,
        /// Degree of parallelism (lanes)
        parallelism: u32,
    },
}

/// Post-authentication database configuration.
/// Contains the full cryptographic configuration after successfully opening a database.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConfigDto {
    /// Database format version (e.g., "KDBX 4.0", "KDBX 3.1")
    pub version: String,
    /// Outer encryption cipher
    pub outer_cipher: OuterCipher,
    /// Inner stream cipher for protecting values in memory
    pub inner_cipher: InnerCipher,
    /// Compression algorithm
    pub compression: Compression,
    /// Key derivation function settings
    pub kdf: KdfSettings,
}
