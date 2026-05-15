// SPDX-License-Identifier: MIT

//! Append-only on-disk JSONL store of encrypted audit frames.
//!
//! One file per Vault, filename derived from [`super::vault_id::hash_vault_path`].
//! Each line is one base64-encoded XChaCha20-Poly1305 frame so appends are
//! O(1) and frames are independently decryptable.
//!
//! Concurrency: each [`AuditLogFile`] owns a process-local mutex. Across
//! processes (and across threads via separate `AuditLogFile` instances), an
//! advisory `flock` on the underlying file serialises writers so a concurrent
//! appender cannot corrupt an in-progress line.

use crate::services::audit::crypto::{decode_frame, encode_frame};
use fs4::fs_std::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("audit log I/O: {0}")]
    Io(String),
}

impl From<std::io::Error> for StorageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

/// A single per-Vault audit log file.
pub struct AuditLogFile {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl AuditLogFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            write_lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends a single encrypted frame as one base64 JSONL line. Takes the
    /// intra-process mutex and an advisory exclusive lock on the file.
    pub fn append(&self, frame: &[u8]) -> Result<(), StorageError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| StorageError::Io("audit append mutex poisoned".into()))?;

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        FileExt::lock_exclusive(&file)?;

        let line = encode_frame(frame);
        let result = (|| -> std::io::Result<()> {
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_data()?;
            Ok(())
        })();

        let _ = FileExt::unlock(&file);
        result.map_err(StorageError::from)
    }

    /// Reads all frames in append order. Missing file returns an empty Vec.
    /// Malformed lines are skipped so a single bad write does not poison reads
    /// (the audit log must never become a `DoS` vector).
    pub fn read_all(&self) -> Result<Vec<Vec<u8>>, StorageError> {
        let file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let reader = BufReader::new(file);
        let mut frames = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(frame) = decode_frame(&line) {
                frames.push(frame);
            }
        }
        Ok(frames)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn append_then_read_returns_frames_in_order() {
        let dir = tempdir().expect("tempdir");
        let log = AuditLogFile::new(dir.path().join("v.jsonl"));

        log.append(b"first").expect("append first");
        log.append(b"second").expect("append second");
        log.append(b"third").expect("append third");

        let frames = log.read_all().expect("read");
        assert_eq!(
            frames,
            vec![b"first".to_vec(), b"second".to_vec(), b"third".to_vec()]
        );
    }

    #[test]
    fn missing_file_reads_as_empty() {
        let dir = tempdir().expect("tempdir");
        let log = AuditLogFile::new(dir.path().join("missing.jsonl"));
        assert!(log.read_all().expect("read").is_empty());
    }

    #[test]
    fn concurrent_appenders_do_not_corrupt_lines() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("v.jsonl");
        let writers_per_thread = 30usize;
        let thread_count = 4usize;

        let mut handles = Vec::new();
        for tid in 0..thread_count {
            let path = path.clone();
            handles.push(thread::spawn(move || {
                // Each thread uses its own AuditLogFile instance so the
                // process-mutex is NOT shared — the advisory file lock is the
                // only thing standing between them.
                let log = Arc::new(AuditLogFile::new(path));
                for i in 0..writers_per_thread {
                    let payload = format!("t{tid}-i{i}");
                    log.append(payload.as_bytes()).expect("append");
                }
            }));
        }
        for h in handles {
            h.join().expect("join");
        }

        let log = AuditLogFile::new(path);
        let frames = log.read_all().expect("read");
        assert_eq!(frames.len(), thread_count * writers_per_thread);

        // Every frame must decode back to a well-formed "t<n>-i<m>" string —
        // a torn write would produce garbage that fails the format check.
        for frame in &frames {
            let s = std::str::from_utf8(frame).expect("utf8");
            assert!(s.starts_with('t'));
            assert!(s.contains("-i"));
        }
    }
}
