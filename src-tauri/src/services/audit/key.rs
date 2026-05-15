// SPDX-License-Identifier: MIT

//! Source of the audit AEAD key.
//!
//! The audit subsystem treats the key as opaque material: it asks for it once
//! per `AuditService` construction. Two backings exist:
//!
//! * [`InMemoryAuditKey`] — used in tests; generates a random key on first
//!   use and caches it for the process lifetime.
//! * [`FileBackedAuditKey`] — production backing for this tracer-bullet
//!   slice. Stores the 32-byte key as `audit/key.bin` under the app's local
//!   data dir so it survives restarts.
//!
//! Both implement [`AuditKey`] so the rest of the audit code is testable
//! without touching the filesystem.

use crate::services::audit::crypto::KEY_LEN;
use rand::rand_core::TryRng;
use rand::rngs::SysRng;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;
use zeroize::Zeroizing;

#[derive(Debug, Error)]
pub enum KeyError {
    #[error("audit key source failed: {0}")]
    Backend(String),
}

/// Source of the audit AEAD key. Implementations must return the same 32-byte
/// key on every call within a process lifetime.
pub trait AuditKey: Send + Sync {
    fn get_or_create(&self) -> Result<[u8; KEY_LEN], KeyError>;
}

/// Process-local audit key. Generates a fresh random key on first call and
/// returns the same bytes on every subsequent call.
pub struct InMemoryAuditKey {
    cached: Mutex<Option<[u8; KEY_LEN]>>,
}

impl InMemoryAuditKey {
    pub fn new() -> Self {
        Self {
            cached: Mutex::new(None),
        }
    }
}

impl Default for InMemoryAuditKey {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditKey for InMemoryAuditKey {
    fn get_or_create(&self) -> Result<[u8; KEY_LEN], KeyError> {
        let mut guard = self
            .cached
            .lock()
            .map_err(|_| KeyError::Backend("mutex poisoned".into()))?;
        if let Some(k) = *guard {
            return Ok(k);
        }
        let mut buf = Zeroizing::new([0u8; KEY_LEN]);
        SysRng
            .try_fill_bytes(&mut buf[..])
            .map_err(|e| KeyError::Backend(e.to_string()))?;
        *guard = Some(*buf);
        Ok(*buf)
    }
}

/// File-backed audit key used in production. Stores the 32 random bytes in a
/// dedicated file under the audit subdir. On the first call generates a key
/// and writes it; on subsequent calls — including those after process restart
/// — reads the existing file and returns the same bytes.
///
/// The file is restricted to user-read/write on Unix; on Windows it relies on
/// the user's home directory ACL.
///
/// NOTE: this is the tracer-bullet implementation. The ADR's long-term goal is
/// to source the key from the OS keychain (Keychain / Credential Manager /
/// Secret Service / Stronghold) so even read access to the filesystem alone
/// does not yield the key. That work is tracked separately; the `AuditKey`
/// trait keeps the swap local.
pub struct FileBackedAuditKey {
    path: PathBuf,
    cached: Mutex<Option<[u8; KEY_LEN]>>,
}

impl FileBackedAuditKey {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cached: Mutex::new(None),
        }
    }
}

impl AuditKey for FileBackedAuditKey {
    fn get_or_create(&self) -> Result<[u8; KEY_LEN], KeyError> {
        let mut guard = self
            .cached
            .lock()
            .map_err(|_| KeyError::Backend("mutex poisoned".into()))?;
        if let Some(k) = *guard {
            return Ok(k);
        }

        if let Ok(bytes) = fs::read(&self.path) {
            if let Ok(arr) = <[u8; KEY_LEN]>::try_from(bytes.as_slice()) {
                *guard = Some(arr);
                return Ok(arr);
            }
            // Length mismatch — treat as missing and overwrite.
        }

        let mut buf = Zeroizing::new([0u8; KEY_LEN]);
        SysRng
            .try_fill_bytes(&mut buf[..])
            .map_err(|e| KeyError::Backend(e.to_string()))?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| KeyError::Backend(e.to_string()))?;
        }
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| KeyError::Backend(e.to_string()))?;
        f.write_all(&buf[..])
            .map_err(|e| KeyError::Backend(e.to_string()))?;
        f.sync_all().map_err(|e| KeyError::Backend(e.to_string()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600));
        }

        *guard = Some(*buf);
        Ok(*buf)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn in_memory_key_returns_stable_value() {
        let source = InMemoryAuditKey::new();
        let a = source.get_or_create().expect("first");
        let b = source.get_or_create().expect("second");
        assert_eq!(a, b);
    }

    #[test]
    fn in_memory_key_has_correct_length_and_entropy() {
        let source = InMemoryAuditKey::new();
        let k = source.get_or_create().expect("key");
        assert_eq!(k.len(), KEY_LEN);
        // Sanity: vanishingly unlikely to be all zeros.
        assert!(k.iter().any(|&b| b != 0));
    }

    #[test]
    fn file_backed_key_persists_across_instances() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("key.bin");

        let first = FileBackedAuditKey::new(path.clone());
        let a = first.get_or_create().expect("first");
        drop(first);

        // Simulate restart: brand new instance reads the persisted file.
        let second = FileBackedAuditKey::new(path);
        let b = second.get_or_create().expect("second");
        assert_eq!(a, b);
    }

    #[test]
    fn file_backed_key_creates_parent_directory() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("audit").join("key.bin");
        let source = FileBackedAuditKey::new(path.clone());
        let _ = source.get_or_create().expect("create");
        assert!(path.exists());
    }
}
