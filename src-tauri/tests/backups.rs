#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::case_sensitive_file_extension_comparisons
)]

//! Integration tests for the backup snapshot module.
//!
//! Tracks acceptance criteria from issue #190: pre-image bytes, filename shape,
//! Unix permissions, first-save skip, fail-closed semantics, symlink resolution.

use mithril_vault_lib::commands::settings::BackupSettings;
#[cfg(unix)]
use mithril_vault_lib::services::kdbx::backups::BackupError;
use mithril_vault_lib::services::kdbx::backups::{snapshot, BACKUP_SUBDIR};
use tempfile::tempdir;

fn enabled() -> BackupSettings {
    BackupSettings { enabled: true }
}

fn disabled() -> BackupSettings {
    BackupSettings { enabled: false }
}

#[test]
fn snapshot_creates_file_with_pre_image_bytes() {
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"pre-image bytes").expect("write source");

    let info = snapshot(&vault_path, &enabled())
        .expect("snapshot ok")
        .expect("snapshot created");

    assert!(info.path.exists(), "snapshot file should exist");
    let bytes = std::fs::read(&info.path).expect("read snapshot");
    assert_eq!(bytes, b"pre-image bytes");
}

#[test]
fn snapshot_filename_matches_documented_pattern() {
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("my-vault.kdbx");
    std::fs::write(&vault_path, b"x").expect("write source");

    let info = snapshot(&vault_path, &enabled())
        .expect("snapshot ok")
        .expect("snapshot created");

    let filename = info
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("filename");
    assert!(
        filename.starts_with("my-vault.kdbx.backup."),
        "unexpected prefix: {filename}"
    );
    assert!(filename.ends_with(".kdbx"), "unexpected suffix: {filename}");
    // Pattern: my-vault.kdbx.backup.YYYYMMDDTHHMMSS.mmmZ.kdbx
    let middle = filename
        .strip_prefix("my-vault.kdbx.backup.")
        .and_then(|s| s.strip_suffix(".kdbx"))
        .expect("middle");
    assert_eq!(middle.len(), 20, "iso ts should be 20 chars: {middle}");
    assert!(middle.ends_with('Z'), "iso ts should end with Z: {middle}");
}

#[test]
fn snapshot_skips_when_source_missing() {
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("never-existed.kdbx");

    let result = snapshot(&vault_path, &enabled()).expect("snapshot ok");
    assert!(result.is_none(), "no snapshot on missing source");

    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    assert!(
        !backup_dir.exists(),
        "no backup dir should be created on first-save skip"
    );
}

#[test]
fn snapshot_skips_when_disabled() {
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    let result = snapshot(&vault_path, &disabled()).expect("snapshot ok");
    assert!(result.is_none(), "disabled snapshot returns None");

    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    assert!(!backup_dir.exists(), "no backup dir when disabled");
}

#[cfg(unix)]
#[test]
fn snapshot_unix_permissions_are_locked_down() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    let info = snapshot(&vault_path, &enabled())
        .expect("snapshot ok")
        .expect("snapshot created");

    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let dir_mode = std::fs::metadata(&backup_dir)
        .expect("dir metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        dir_mode, 0o700,
        "backup dir should be 0700, got {dir_mode:o}"
    );

    let file_mode = std::fs::metadata(&info.path)
        .expect("file metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        file_mode, 0o600,
        "backup file should be 0600, got {file_mode:o}"
    );
}

#[cfg(unix)]
#[test]
fn snapshot_fails_closed_when_directory_cannot_be_created() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    // Make the parent directory read-only so .kdbx-backups/ cannot be created.
    let mut perms = std::fs::metadata(dir.path())
        .expect("dir meta")
        .permissions();
    let original = perms.mode();
    perms.set_mode(0o500);
    std::fs::set_permissions(dir.path(), perms).expect("set ro");

    let result = snapshot(&vault_path, &enabled());

    // Restore so tempdir cleanup works.
    let mut restore = std::fs::metadata(dir.path())
        .expect("dir meta")
        .permissions();
    restore.set_mode(original);
    std::fs::set_permissions(dir.path(), restore).expect("restore");

    match result {
        Err(BackupError::BackupFailed { path, .. }) => {
            assert!(
                path.ends_with(BACKUP_SUBDIR),
                "BackupFailed path should name the backup directory, got {path:?}"
            );
        }
        other => panic!("expected BackupFailed, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn snapshot_follows_symlinks_to_target_bytes() {
    let dir = tempdir().expect("tempdir");
    let real_vault = dir.path().join("real-vault.kdbx");
    let symlink = dir.path().join("vault.kdbx");
    std::fs::write(&real_vault, b"target bytes").expect("write target");
    std::os::unix::fs::symlink(&real_vault, &symlink).expect("symlink");

    let info = snapshot(&symlink, &enabled())
        .expect("snapshot ok")
        .expect("snapshot created");

    let bytes = std::fs::read(&info.path).expect("read snapshot");
    assert_eq!(bytes, b"target bytes");
}

#[cfg(unix)]
#[test]
fn snapshot_rehardens_backup_dir_to_0700_if_left_loose() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    // Simulate a pre-existing .kdbx-backups/ created with broader mode
    // (e.g. by an older version of the app or a manual `mkdir`).
    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    std::fs::create_dir_all(&backup_dir).expect("create dir");
    std::fs::set_permissions(&backup_dir, std::fs::Permissions::from_mode(0o755))
        .expect("set loose");

    snapshot(&vault_path, &enabled())
        .expect("snapshot ok")
        .expect("snapshot created");

    let mode = std::fs::metadata(&backup_dir)
        .expect("dir meta")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        mode, 0o700,
        "pre-existing backup dir should be re-hardened to 0700, got {mode:o}"
    );
}

#[cfg(unix)]
#[test]
fn snapshot_rejects_symlinked_backup_directory() {
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    // Plant a symlink at the .kdbx-backups/ path pointing somewhere else.
    let elsewhere = tempdir().expect("tempdir 2");
    let backup_link = dir.path().join(BACKUP_SUBDIR);
    std::os::unix::fs::symlink(elsewhere.path(), &backup_link).expect("symlink");

    let result = snapshot(&vault_path, &enabled());
    assert!(
        matches!(result, Err(BackupError::BackupFailed { .. })),
        "symlinked backup directory must abort the snapshot, got {result:?}"
    );

    // Confirm we did not write into the symlink target.
    let exfiltrated: Vec<_> = std::fs::read_dir(elsewhere.path())
        .expect("read elsewhere")
        .filter_map(Result::ok)
        .collect();
    assert!(
        exfiltrated.is_empty(),
        "no bytes should have been written via the symlink"
    );
}

#[test]
fn snapshot_handles_same_millisecond_collision() {
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"v1").expect("write source");

    let first = snapshot(&vault_path, &enabled())
        .expect("snapshot ok")
        .expect("created");
    // Overwrite source to bump mtime; rapid second call may hit same ms.
    std::fs::write(&vault_path, b"v2").expect("write source");
    let second = snapshot(&vault_path, &enabled())
        .expect("snapshot ok")
        .expect("created");

    assert_ne!(first.path, second.path, "collision should bump filename");
    let bytes = std::fs::read(&second.path).expect("read");
    assert_eq!(bytes, b"v2");
}
