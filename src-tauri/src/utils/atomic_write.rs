// SPDX-License-Identifier: MIT

//! Atomic file write utility for safe database persistence.
//!
//! This module provides an atomic write pattern that ensures system interruptions
//! cannot corrupt existing database files:
//! 1. Create temp file with `.tmp` extension in same directory
//! 2. Set secure permissions (0600 on Unix)
//! 3. Write content via closure
//! 4. Call `sync_all()` for durability
//! 5. Atomic rename to target
//! 6. Cleanup temp file on any failure

use crate::models::error::AppError;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Options for atomic write operations
#[derive(Debug, Clone, Default)]
pub struct AtomicWriteOptions {
    /// If true, preserve the original file's permissions when overwriting.
    /// If false, set secure default permissions (0600 on Unix).
    pub preserve_permissions: bool,
}

/// Performs an atomic write operation using the temp file + rename pattern.
///
/// # Arguments
/// * `target_path` - The final destination path for the file
/// * `options` - Configuration options for the write operation
/// * `write_fn` - A closure that writes content to the provided `File`
///
/// # Security
/// - On Unix, new files are created with 0600 permissions (owner read/write only)
/// - Temp files are cleaned up on failure to avoid leaving partial data
///
/// # Atomicity
/// - Uses `rename()` for atomic replacement on POSIX systems
/// - Calls `sync_all()` before rename to ensure data is persisted
///
/// # Errors
/// Returns `AppError::AtomicWrite` if temp file creation or rename fails,
/// or `AppError::SyncFailed` if disk sync fails.
pub fn atomic_write<F>(
    target_path: &str,
    options: &AtomicWriteOptions,
    write_fn: F,
) -> Result<(), AppError>
where
    F: FnOnce(&mut File) -> Result<(), AppError>,
{
    let target = Path::new(target_path);
    let parent = target.parent().ok_or_else(|| {
        AppError::AtomicWrite(format!(
            "Cannot determine parent directory for: {target_path}"
        ))
    })?;

    // Generate temp file path in the same directory (required for atomic rename)
    let temp_filename = format!(
        ".{}.tmp",
        target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("database")
    );
    let temp_path = parent.join(&temp_filename);

    // Capture existing permissions if needed
    #[cfg(unix)]
    let existing_permissions = if options.preserve_permissions {
        fs::metadata(target).ok().map(|m| m.permissions())
    } else {
        None
    };

    // Create temp file with secure permissions
    let mut file = create_secure_file(&temp_path)?;

    // Write content via the provided closure
    let write_result = write_fn(&mut file);

    // Handle write failure - cleanup temp file
    if let Err(e) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    // Ensure data is flushed to the OS buffer
    if let Err(e) = file.flush() {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::SyncFailed(format!("Failed to flush file: {e}")));
    }

    // Sync to disk for durability
    if let Err(e) = file.sync_all() {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::SyncFailed(format!(
            "Failed to sync file to disk: {e}"
        )));
    }

    // Drop the file handle before rename
    drop(file);

    // Atomic rename
    #[cfg(windows)]
    if target.exists() {
        // Windows rename fails if the destination exists; remove first.
        if let Err(e) = fs::remove_file(target) {
            let _ = fs::remove_file(&temp_path);
            return Err(AppError::AtomicWrite(format!(
                "Failed to remove existing target file: {e}"
            )));
        }
    }

    if let Err(e) = fs::rename(&temp_path, target) {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::AtomicWrite(format!(
            "Failed to rename temp file to target: {e}"
        )));
    }

    // Restore original permissions if requested
    #[cfg(unix)]
    if let Some(perms) = existing_permissions {
        // Best effort - don't fail if we can't restore permissions
        let _ = fs::set_permissions(target, perms);
    }

    Ok(())
}

/// Creates a file with secure permissions (0600 on Unix)
#[cfg(unix)]
fn create_secure_file(path: &Path) -> Result<File, AppError> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600) // Owner read/write only
        .open(path)
        .map_err(|e| AppError::AtomicWrite(format!("Failed to create temp file: {e}")))
}

/// Creates a file (Windows - no special permissions)
#[cfg(not(unix))]
fn create_secure_file(path: &Path) -> Result<File, AppError> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|e| AppError::AtomicWrite(format!("Failed to create temp file: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write_creates_file() -> Result<(), AppError> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.kdbx");

        atomic_write(
            &file_path.to_string_lossy(),
            &AtomicWriteOptions::default(),
            |file| {
                file.write_all(b"test content")
                    .map_err(|e| AppError::Io(e.to_string()))
            },
        )?;

        assert!(file_path.exists(), "Target file should exist");

        let mut content = String::new();
        File::open(&file_path)?.read_to_string(&mut content)?;
        assert_eq!(content, "test content");
        Ok(())
    }

    #[test]
    fn test_atomic_write_no_temp_file_on_success() -> Result<(), AppError> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.kdbx");
        let temp_path = dir.path().join(".test.kdbx.tmp");

        atomic_write(
            &file_path.to_string_lossy(),
            &AtomicWriteOptions::default(),
            |file| {
                file.write_all(b"test content")
                    .map_err(|e| AppError::Io(e.to_string()))
            },
        )?;

        assert!(
            !temp_path.exists(),
            "Temp file should not exist after success"
        );
        Ok(())
    }

    #[test]
    fn test_atomic_write_cleans_up_on_failure() -> Result<(), AppError> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.kdbx");
        let temp_path = dir.path().join(".test.kdbx.tmp");

        let result = atomic_write(
            &file_path.to_string_lossy(),
            &AtomicWriteOptions::default(),
            |_file| Err(AppError::Kdbx("Simulated write failure".to_string())),
        );

        assert!(result.is_err(), "Should fail on write error");
        assert!(!temp_path.exists(), "Temp file should be cleaned up");
        assert!(!file_path.exists(), "Target file should not exist");
        Ok(())
    }

    #[test]
    fn test_atomic_write_overwrites_existing() -> Result<(), AppError> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.kdbx");

        // Create initial file
        fs::write(&file_path, "original content")?;

        // Overwrite with atomic write
        atomic_write(
            &file_path.to_string_lossy(),
            &AtomicWriteOptions::default(),
            |file| {
                file.write_all(b"new content")
                    .map_err(|e| AppError::Io(e.to_string()))
            },
        )?;

        let content = fs::read_to_string(&file_path)?;
        assert_eq!(content, "new content");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_atomic_write_sets_unix_permissions() -> Result<(), AppError> {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir()?;
        let file_path = dir.path().join("secure.kdbx");

        atomic_write(
            &file_path.to_string_lossy(),
            &AtomicWriteOptions::default(),
            |file| {
                file.write_all(b"secure content")
                    .map_err(|e| AppError::Io(e.to_string()))
            },
        )?;

        let metadata = fs::metadata(&file_path)?;
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "File should have 0600 permissions");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_atomic_write_preserves_existing_permissions() -> Result<(), AppError> {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir()?;
        let file_path = dir.path().join("preserved.kdbx");

        // Create initial file with custom permissions
        fs::write(&file_path, "original")?;
        let mut perms = fs::metadata(&file_path)?.permissions();
        perms.set_mode(0o640);
        fs::set_permissions(&file_path, perms)?;

        // Overwrite with preserve_permissions = true
        atomic_write(
            &file_path.to_string_lossy(),
            &AtomicWriteOptions {
                preserve_permissions: true,
            },
            |file| {
                file.write_all(b"new content")
                    .map_err(|e| AppError::Io(e.to_string()))
            },
        )?;

        let metadata = fs::metadata(&file_path)?;
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o640, "Original permissions should be preserved");
        Ok(())
    }

    #[test]
    fn test_atomic_write_preserves_original_on_write_failure() -> Result<(), AppError> {
        let dir = tempdir()?;
        let file_path = dir.path().join("preserved-on-failure.kdbx");

        // Create initial file
        fs::write(&file_path, "original content")?;

        // Attempt atomic write that fails
        let result = atomic_write(
            &file_path.to_string_lossy(),
            &AtomicWriteOptions::default(),
            |_file| Err(AppError::Kdbx("Simulated failure".to_string())),
        );

        assert!(result.is_err());

        // Original file should be unchanged
        let content = fs::read_to_string(&file_path)?;
        assert_eq!(
            content, "original content",
            "Original file should be preserved on failure"
        );
        Ok(())
    }
}
