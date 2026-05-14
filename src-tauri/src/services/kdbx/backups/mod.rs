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
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt::Write as _;
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

/// Snapshot classification, derived from the filename pattern.
///
/// `Auto` covers snapshots created by the save-side and open-side hooks.
/// `Manual` is reserved for the future manual-backup slice (parent #61); the
/// listing surfaces it now so the UI doesn't need to change shape later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupKind {
    Auto,
    Manual,
}

/// Listing row for the Settings → Backups table.
///
/// Built by walking the resolved backup directory for a single Vault and
/// parsing each file's name. The timestamp is the one encoded in the
/// filename (not the file mtime — mtime is preserved by snapshot writes and
/// would order files arbitrarily after rotations).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupListEntry {
    pub path: PathBuf,
    /// ISO-8601 UTC timestamp parsed from the filename (e.g. `2026-05-12T14:30:45.123Z`).
    pub timestamp: String,
    pub size_bytes: u64,
    pub kind: BackupKind,
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

    let backup_dir = resolve_backup_dir(source, settings)?;
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

/// Open-side snapshot hook (issue #193).
///
/// Behaviour differs from [`snapshot`] in two ways:
///
/// - Gated on `settings.on_open` (default off). The `enabled` flag is still
///   the master switch, so toggling off all backups disables this hook too.
/// - Deduplicates against the latest existing snapshot for this Vault by
///   direct byte-for-byte comparison (after a cheap length check). If the
///   snapshot already captures the current bytes, no new snapshot is taken
///   — this prevents flooding the rotation bucket when a user locks and
///   unlocks an unchanged Vault repeatedly. See `content_matches` for why
///   metadata proxies (size + mtime) aren't reliable on cross-filesystem
///   overrides. The snapshot's mtime is still stamped to the source's so
///   that browsing the backup folder shows useful timestamps; dedup does
///   not depend on it.
///
/// Failure semantics are the caller's concern. This function still returns
/// `Err(BackupFailed)` on I/O failure; the open-path command converts that
/// into a non-blocking `backup-warning` event so the unlock itself never
/// fails because of a backup problem.
pub fn snapshot_on_open(
    source: &Path,
    settings: &BackupSettings,
) -> Result<Option<BackupInfo>, BackupError> {
    if !settings.enabled || !settings.on_open {
        return Ok(None);
    }

    // Dedup before taking the snapshot. If the latest existing snapshot
    // already matches the source's size+mtime there is nothing new to
    // capture — silently skip so a locked/unlocked unchanged Vault doesn't
    // burn a rotation slot per cycle.
    let source_exists = source.try_exists().map_err(|e| BackupError::BackupFailed {
        path: source.to_path_buf(),
        source: e,
    })?;
    if source_exists {
        let backup_dir = resolve_backup_dir(source, settings)?;
        // Run the symlink guard up front so the dedup short-circuit cannot
        // silently bypass it. Without this check, an attacker who can plant
        // a symlink at the backup-dir path AND a matching-metadata file at
        // the link's target would suppress every open-side snapshot and
        // every `backup-warning` event for as long as the symlink survives.
        reject_symlinked_backup_dir(&backup_dir).map_err(|e| BackupError::BackupFailed {
            path: backup_dir.clone(),
            source: e,
        })?;
        if let Some(vault_filename) = source.file_name().and_then(|n| n.to_str()) {
            if let Some(latest) = latest_snapshot_for(&backup_dir, vault_filename) {
                if content_matches(&latest, source).unwrap_or(false) {
                    return Ok(None);
                }
            }
        }
    }

    let info = snapshot(source, settings)?;
    if let Some(info) = info.as_ref() {
        stamp_source_mtime(&info.path, source).map_err(|e| BackupError::BackupFailed {
            path: info.path.clone(),
            source: e,
        })?;
    }
    Ok(info)
}

/// Enumerates every snapshot belonging to `source` inside the resolved
/// backup directory. The listing covers both auto- and manual-snapshot
/// naming patterns; foreign-Vault snapshots and unrelated files are
/// skipped.
///
/// Returns `Ok(vec![])` when the backup directory does not yet exist (no
/// snapshot has ever been taken for this Vault).
pub fn list_for(
    source: &Path,
    settings: &BackupSettings,
) -> Result<Vec<BackupListEntry>, BackupError> {
    let backup_dir = resolve_backup_dir(source, settings)?;
    // Apply the same symlink rejection the snapshot writer uses — a symlink
    // at the backup dir would otherwise let `read_dir` enumerate an arbitrary
    // target directory and surface those files as this Vault's backups in
    // the Settings UI.
    reject_symlinked_backup_dir(&backup_dir).map_err(|e| BackupError::BackupFailed {
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

    let read_dir = match fs::read_dir(&backup_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(BackupError::BackupFailed {
                path: backup_dir,
                source: e,
            })
        }
    };

    let mut out: Vec<BackupListEntry> = Vec::new();
    for entry in read_dir.filter_map(Result::ok) {
        if !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some((kind, ts)) = classify_snapshot_name(&name, vault_filename) else {
            continue;
        };
        let size_bytes = entry.metadata().map_or(0, |m| m.len());
        out.push(BackupListEntry {
            path: entry.path(),
            timestamp: ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            size_bytes,
            kind,
        });
    }
    out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(out)
}

/// Parses a snapshot filename into `(kind, timestamp)` for one specific
/// Vault. Returns `None` for foreign-Vault snapshots and non-snapshot files.
///
/// Manual snapshots use the reserved `.backup.manual.` infix; auto snapshots
/// use plain `.backup.`. The manual check runs first because a manual name
/// also matches the auto parser if we strip the `manual.` segment off.
fn classify_snapshot_name(
    name: &str,
    vault_filename: &str,
) -> Option<(BackupKind, chrono::DateTime<Utc>)> {
    let manual_prefix = format!("{vault_filename}.backup.manual.");
    if let Some(rest) = name.strip_prefix(&manual_prefix) {
        let ts_str = rest.strip_suffix(".kdbx")?;
        let parsed = chrono::NaiveDateTime::parse_from_str(ts_str, "%Y%m%dT%H%M%S%.3fZ").ok()?;
        return Some((
            BackupKind::Manual,
            chrono::TimeZone::from_utc_datetime(&Utc, &parsed),
        ));
    }
    let (parsed_vault, ts) = filename::parse_backup_filename(name)?;
    (parsed_vault == vault_filename).then_some((BackupKind::Auto, ts))
}

/// Finds the most recent auto-snapshot for `vault_filename` inside `dir`,
/// using the timestamp parsed out of the filename. Returns `None` when the
/// directory does not exist or holds no snapshots for this Vault.
fn latest_snapshot_for(dir: &Path, vault_filename: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            let (vault, ts) = filename::parse_backup_filename(&name)?;
            (vault == vault_filename).then_some((ts, entry.path()))
        })
        .max_by_key(|(ts, _)| *ts)
        .map(|(_, p)| p)
}

/// Decides whether the latest existing snapshot already captures what the
/// source currently holds, by comparing bytes directly after a cheap length
/// check.
///
/// File-metadata proxies (size + mtime) don't work reliably here: stamping
/// the snapshot's mtime to the source's gets rounded on coarser destination
/// filesystems (FAT/exFAT round to 2 s; many SMB shares are similar), and a
/// fuzzy mtime match would in turn mask a real same-length content change
/// — KDBX writes encrypted blocks at fixed sizes, and sync tools that
/// preserve mtime can land identical (len, mtime) on actually-changed bytes.
/// A direct compare sidesteps both failure modes.
///
/// Comparison is streamed through two `BufReader`s so memory stays bounded
/// regardless of vault size — KDBX databases with attachments/history can
/// be tens of MB, and we run on the open path right after KDF; allocating
/// two full file copies just to decide whether to skip a backup would cause
/// a needless spike. Both files contain encrypted on-disk bytes, so the
/// short-lived buffers don't introduce new sensitive-data exposure.
fn content_matches(snapshot: &Path, source: &Path) -> io::Result<bool> {
    use io::BufRead;
    const READ_CAPACITY: usize = 64 * 1024;

    let snap_meta = fs::metadata(snapshot)?;
    let src_meta = fs::metadata(source)?;
    if snap_meta.len() != src_meta.len() {
        return Ok(false);
    }

    let mut snap = io::BufReader::with_capacity(READ_CAPACITY, fs::File::open(snapshot)?);
    let mut src = io::BufReader::with_capacity(READ_CAPACITY, fs::File::open(source)?);
    loop {
        let snap_buf = snap.fill_buf()?;
        let src_buf = src.fill_buf()?;
        if snap_buf.is_empty() && src_buf.is_empty() {
            return Ok(true);
        }
        let chunk = snap_buf.len().min(src_buf.len());
        // Defensive: lengths matched in the size check above, so reaching
        // EOF on only one side means the file is being truncated under us.
        // Bail out as "no match" rather than reading garbage.
        if chunk == 0 {
            return Ok(false);
        }
        if snap_buf[..chunk] != src_buf[..chunk] {
            return Ok(false);
        }
        snap.consume(chunk);
        src.consume(chunk);
    }
}

/// Stamps `snapshot_path`'s mtime to match `source`'s mtime so that browsing
/// the backup folder in a file manager surfaces the *source's* last-saved
/// time rather than "when the snapshot was written" (which would just be
/// "every time you opened the Vault"). Not load-bearing for dedup — that
/// uses `content_matches`.
fn stamp_source_mtime(snapshot_path: &Path, source: &Path) -> io::Result<()> {
    let source_meta = fs::metadata(source)?;
    let source_mtime = source_meta.modified()?;
    let file = fs::OpenOptions::new().write(true).open(snapshot_path)?;
    file.set_modified(source_mtime)?;
    Ok(())
}

/// Public façade over [`resolve_backup_dir`] for callers outside this
/// module — used by the `delete_backup` command to compute the safety
/// boundary for a given open vault.
pub fn resolved_backup_dir(
    source: &Path,
    settings: &BackupSettings,
) -> Result<PathBuf, BackupError> {
    resolve_backup_dir(source, settings)
}

/// Resolves the directory that snapshots should be written to. When
/// `settings.directory` is set, snapshots are isolated per source vault
/// inside it (`<override>/<basename>-<hash>/`) so two vaults sharing a
/// custom backup directory do not contaminate each other's rotation
/// history. Otherwise the per-Vault `.kdbx-backups/` sibling subdir is
/// used — that one is already per-vault by virtue of being a sibling.
fn resolve_backup_dir(source: &Path, settings: &BackupSettings) -> Result<PathBuf, BackupError> {
    if let Some(override_path) = settings.directory.as_deref() {
        if !override_path.is_empty() {
            let isolation = vault_isolation_segment(source)?;
            return Ok(PathBuf::from(override_path).join(isolation));
        }
    }
    let parent = source.parent().ok_or_else(|| BackupError::BackupFailed {
        path: source.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "source has no parent"),
    })?;
    Ok(parent.join(BACKUP_SUBDIR))
}

/// Builds a stable per-vault directory segment: `<basename>-<short-hash>`.
///
/// The hash is SHA-256 of the source's canonicalized absolute path, hex
/// truncated to 16 chars. Canonicalization resolves symlinks and `..`
/// components so two routes to the same file collapse to one history;
/// it falls back to the raw path when canonicalization fails (e.g. on
/// platforms or filesystems where canonicalize misbehaves) so isolation
/// is never weaker than "path string equality".
///
/// The basename is included verbatim for human recognisability when
/// browsing the override directory.
fn vault_isolation_segment(source: &Path) -> Result<String, BackupError> {
    let basename =
        source
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| BackupError::BackupFailed {
                path: source.to_path_buf(),
                source: io::Error::new(io::ErrorKind::InvalidInput, "source has no filename"),
            })?;
    let canonical = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_os_str().as_encoded_bytes());
    let digest = hasher.finalize();
    let mut hash_hex = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        let _ = write!(&mut hash_hex, "{byte:02x}");
    }
    Ok(format!("{basename}-{hash_hex}"))
}

/// Public façade over [`reject_symlinked_backup_dir`] for callers outside
/// this module. Used by the delete-backup command to refuse path-safety
/// resolution against a symlinked backup directory — without it, a planted
/// symlink could shift the allowed delete boundary to the symlink target
/// and let an attacker remove files outside the real backup directory.
pub fn assert_backup_dir_not_symlinked(dir: &Path) -> io::Result<()> {
    reject_symlinked_backup_dir(dir)
}

/// Rejects a symlink at the backup path so a stale or hostile link cannot
/// redirect snapshot bytes outside the vault folder. `symlink_metadata` does
/// not follow links. Shared between the save-side `snapshot` (via
/// `ensure_backup_dir`) and the open-side `snapshot_on_open` so the dedup
/// short-circuit cannot silently bypass the guard.
fn reject_symlinked_backup_dir(dir: &Path) -> io::Result<()> {
    if let Ok(meta) = fs::symlink_metadata(dir) {
        if meta.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "backup directory path is a symlink",
            ));
        }
    }
    Ok(())
}

fn ensure_backup_dir(dir: &Path) -> io::Result<()> {
    reject_symlinked_backup_dir(dir)?;
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
