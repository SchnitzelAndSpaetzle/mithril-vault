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
