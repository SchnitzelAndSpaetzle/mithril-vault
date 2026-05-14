// SPDX-License-Identifier: MIT

//! Pre-save Vault snapshot module.
//!
//! Owns directory resolution, snapshot filename construction, and the snapshot
//! write itself. See parent issue #61 for the full design and #190 for this slice.

pub mod filename;
pub(crate) mod rotation;

use crate::commands::settings::BackupSettings;
use crate::utils::atomic_write::{atomic_write, AtomicWriteOptions};
use chrono::Utc;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Sibling subdirectory that holds snapshot files for a Vault.
pub const BACKUP_SUBDIR: &str = ".kdbx-backups";

/// Outcome of a successful snapshot.
#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
}

/// Failure modes for snapshot creation.
#[derive(Debug, Error)]
pub enum BackupError {
    #[error("Backup failed for {path}: {source}")]
    BackupFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Creates a pre-image snapshot of the on-disk Vault.
///
/// Returns `Ok(None)` when the source path does not exist yet — the first save
/// of a brand-new Vault (or the first save after `save_as` to a fresh path)
/// has no pre-image to capture and must not error.
///
/// Returns `Ok(Some(info))` on a successful snapshot, or `Err(BackupFailed)`
/// if the directory cannot be prepared or the snapshot cannot be written.
pub fn snapshot(
    source: &Path,
    settings: &BackupSettings,
) -> Result<Option<BackupInfo>, BackupError> {
    if !settings.enabled {
        return Ok(None);
    }

    // First-save (and first-save-after-save-as) skip: source does not exist yet.
    // `try_exists` is used so a real I/O failure surfaces rather than being
    // silently treated as "not yet there".
    let exists = source.try_exists().map_err(|e| BackupError::BackupFailed {
        path: source.to_path_buf(),
        source: e,
    })?;
    if !exists {
        return Ok(None);
    }

    let parent = source.parent().ok_or_else(|| BackupError::BackupFailed {
        path: source.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "source has no parent"),
    })?;
    let backup_dir = parent.join(BACKUP_SUBDIR);
    ensure_backup_dir(&backup_dir).map_err(|e| BackupError::BackupFailed {
        path: backup_dir.clone(),
        source: e,
    })?;

    let vault_filename =
        source
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| BackupError::BackupFailed {
                path: source.to_path_buf(),
                source: io::Error::new(io::ErrorKind::InvalidInput, "source has no filename"),
            })?;

    let existing = read_existing_basenames(&backup_dir);
    let ts = filename::next_free_timestamp(vault_filename, Utc::now(), &existing);
    let backup_name = filename::make_backup_filename(vault_filename, ts);
    let backup_path = backup_dir.join(&backup_name);
    let backup_path_str = backup_path.to_string_lossy().into_owned();

    let source_owned = source.to_path_buf();
    atomic_write(
        &backup_path_str,
        &AtomicWriteOptions {
            preserve_permissions: false,
        },
        |file| {
            let mut src = fs::File::open(&source_owned).map_err(|e| {
                crate::dto::error::AppError::Io(format!("Failed to open source for snapshot: {e}"))
            })?;
            io::copy(&mut src, file).map_err(|e| {
                crate::dto::error::AppError::Io(format!("Failed to copy snapshot bytes: {e}"))
            })?;
            Ok(())
        },
    )
    .map_err(|e| BackupError::BackupFailed {
        path: backup_path.clone(),
        source: io::Error::other(e.to_string()),
    })?;

    // Trim only after a successful new-snapshot write so a failed snapshot
    // never deletes existing backups. Rotation is keyed on the source Vault's
    // basename: two Vaults sharing a backup directory rotate independently.
    rotation::rotate(&backup_dir, vault_filename, settings.max_versions).map_err(|e| {
        BackupError::BackupFailed {
            path: backup_dir.clone(),
            source: e,
        }
    })?;

    Ok(Some(BackupInfo { path: backup_path }))
}

fn ensure_backup_dir(dir: &Path) -> io::Result<()> {
    // Reject a symlink at the backup path so a stale or hostile link cannot
    // redirect snapshot bytes outside the vault folder. `symlink_metadata`
    // does not follow links.
    if let Ok(meta) = fs::symlink_metadata(dir) {
        if meta.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "backup directory path is a symlink",
            ));
        }
    }
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }
    // Always (re-)apply 0700 on Unix — a directory left over from an earlier
    // app version or created by another process may have broader permissions
    // that would leak vault filenames/timestamps.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn read_existing_basenames(dir: &Path) -> HashSet<String> {
    fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
