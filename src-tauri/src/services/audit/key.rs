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
use std::io::{self, Write};
use std::path::{Path, PathBuf};
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

        // Read path: only `NotFound` means "create new". Any other error
        // (`PermissionDenied`, transient I/O, …) propagates instead of
        // silently rotating the key and orphaning the existing audit log.
        // A malformed file (wrong length) is also a hard error — silently
        // overwriting it would lose history just as decisively as rotation.
        match read_key_file(&self.path) {
            Ok(Some(arr)) => {
                *guard = Some(arr);
                return Ok(arr);
            }
            Ok(None) => {} // file genuinely missing — fall through to create
            Err(e) => return Err(KeyError::Backend(e.to_string())),
        }

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| KeyError::Backend(e.to_string()))?;
        }

        let mut buf = Zeroizing::new([0u8; KEY_LEN]);
        SysRng
            .try_fill_bytes(&mut buf[..])
            .map_err(|e| KeyError::Backend(e.to_string()))?;

        // Atomic create-and-restrict-mode in one syscall: `create_new`
        // closes the TOCTOU window where a concurrent process could see a
        // half-written or default-permissioned file, and on Unix the
        // restrictive mode is applied at file creation (subject to umask
        // intersection, which can only further restrict the mode).
        let mut open_opts = fs::OpenOptions::new();
        open_opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            open_opts.mode(0o600);
        }

        let mut f = match open_opts.open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                // Lost a race with a concurrent creator. Read what they
                // wrote and adopt their key — never overwrite, never
                // rotate.
                return match read_key_file(&self.path) {
                    Ok(Some(arr)) => {
                        *guard = Some(arr);
                        Ok(arr)
                    }
                    Ok(None) => Err(KeyError::Backend(
                        "audit key file appeared then vanished during creation race".into(),
                    )),
                    Err(e) => Err(KeyError::Backend(e.to_string())),
                };
            }
            Err(e) => return Err(KeyError::Backend(e.to_string())),
        };
        f.write_all(&buf[..])
            .map_err(|e| KeyError::Backend(e.to_string()))?;
        f.sync_all().map_err(|e| KeyError::Backend(e.to_string()))?;

        *guard = Some(*buf);
        Ok(*buf)
    }
}

/// Reads the audit key from `path`. Returns `Ok(None)` only when the file
/// genuinely does not exist; any other I/O error propagates so the caller
/// does not mistake a transient read failure for "first run, create new key"
/// and rotate the user's audit history into oblivion.
fn read_key_file(path: &Path) -> io::Result<Option<[u8; KEY_LEN]>> {
    match fs::read(path) {
        Ok(bytes) => match <[u8; KEY_LEN]>::try_from(bytes.as_slice()) {
            Ok(arr) => Ok(Some(arr)),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "audit key file at {} is {} bytes; expected {KEY_LEN}",
                    path.display(),
                    bytes.len()
                ),
            )),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
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

    /// Read errors other than `NotFound` must NOT rotate the key. Stage a
    /// directory at the key path so `fs::read` returns a non-`NotFound`
    /// error; the call must fail rather than silently regenerating.
    #[test]
    fn non_not_found_read_error_propagates_instead_of_rotating() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("key.bin");
        // Stage a directory where the file should be: `fs::read` returns
        // `IsADirectory` / `Other`, never `NotFound`.
        fs::create_dir(&path).expect("stage dir");

        let source = FileBackedAuditKey::new(path);
        assert!(matches!(source.get_or_create(), Err(KeyError::Backend(_))));
    }

    /// A wrong-length key file is a hard error, never a silent overwrite —
    /// the user's existing audit log would otherwise become permanently
    /// unreadable on the next process start.
    #[test]
    fn malformed_key_file_propagates_instead_of_rotating() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("key.bin");
        fs::write(&path, b"not 32 bytes").expect("stage malformed key");

        let source = FileBackedAuditKey::new(path);
        assert!(matches!(source.get_or_create(), Err(KeyError::Backend(_))));
    }

    /// Concurrent creators must converge on the same key. Spawn several
    /// threads, each with its own `FileBackedAuditKey` instance pointing
    /// at the same path so the per-instance Mutex cannot serialise them,
    /// then assert every returned key matches.
    #[test]
    fn concurrent_creators_converge_on_one_key() {
        use std::sync::Barrier;
        use std::thread;

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("key.bin");

        let thread_count = 8;
        let barrier = std::sync::Arc::new(Barrier::new(thread_count));

        let mut handles = Vec::with_capacity(thread_count);
        for _ in 0..thread_count {
            let path = path.clone();
            let barrier = std::sync::Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                let source = FileBackedAuditKey::new(path);
                // Release all threads at roughly the same instant so at
                // least some of them hit the `create_new` race window.
                barrier.wait();
                source.get_or_create().expect("get_or_create")
            }));
        }

        let keys: Vec<[u8; KEY_LEN]> = handles.into_iter().map(|h| h.join().expect("join")).collect();
        let first = keys[0];
        for k in &keys[1..] {
            assert_eq!(*k, first, "concurrent creators diverged on the audit key");
        }

        // On-disk state must match the in-memory consensus.
        let on_disk = fs::read(&path).expect("read");
        assert_eq!(on_disk.as_slice(), first.as_slice());
    }

    #[cfg(unix)]
    #[test]
    fn newly_created_key_file_is_owner_read_write_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("key.bin");
        let source = FileBackedAuditKey::new(path.clone());
        let _ = source.get_or_create().expect("create");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        // The umask can only further restrict bits we requested; 0o600 is
        // already the most-restrictive practical setting, so equality holds
        // on every realistic CI host.
        assert_eq!(mode, 0o600, "audit key file should be 0o600, was {mode:o}");
    }
}
