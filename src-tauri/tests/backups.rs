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
    BackupSettings {
        enabled: true,
        ..BackupSettings::default()
    }
}

fn disabled() -> BackupSettings {
    BackupSettings {
        enabled: false,
        ..BackupSettings::default()
    }
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
fn rotation_caps_snapshots_at_max_versions() {
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let settings = BackupSettings {
        enabled: true,
        max_versions: 10,
        directory: None,
    };

    // Twelve consecutive snapshots: each save mutates the source so the
    // pre-image bytes differ and there's something distinct to capture.
    for i in 0..12u32 {
        std::fs::write(&vault_path, format!("v{i}").as_bytes()).expect("write source");
        snapshot(&vault_path, &settings)
            .expect("snapshot ok")
            .expect("created");
    }

    let kept: Vec<_> = std::fs::read_dir(&backup_dir)
        .expect("read backup dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .collect();
    assert_eq!(
        kept.len(),
        10,
        "after 12 saves with max_versions=10, exactly 10 snapshots remain"
    );
}

#[test]
fn rotation_retains_newest_snapshots_not_oldest() {
    // Build 5 snapshots with cap=3 and confirm the *newest* 3 survive.
    // Without this we'd silently keep the oldest 3 and lose every recent
    // pre-image, which is the opposite of useful.
    use mithril_vault_lib::services::kdbx::backups::filename::parse_backup_filename;

    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let settings = BackupSettings {
        enabled: true,
        max_versions: 3,
        directory: None,
    };

    let mut all_timestamps = Vec::new();
    for i in 0..5u32 {
        std::fs::write(&vault_path, format!("v{i}").as_bytes()).expect("write source");
        let info = snapshot(&vault_path, &settings)
            .expect("snapshot ok")
            .expect("created");
        let name = info.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let (_, ts) = parse_backup_filename(name).expect("parse");
        all_timestamps.push(ts);
    }

    let surviving_timestamps: Vec<_> = std::fs::read_dir(&backup_dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            parse_backup_filename(&n).map(|(_, ts)| ts)
        })
        .collect();

    let mut newest_three = all_timestamps.clone();
    newest_three.sort();
    let newest_three: Vec<_> = newest_three.into_iter().rev().take(3).collect();

    for ts in &newest_three {
        assert!(
            surviving_timestamps.contains(ts),
            "newest snapshot {ts:?} should have been retained"
        );
    }
    assert_eq!(
        surviving_timestamps.len(),
        3,
        "exactly cap=3 snapshots should remain"
    );
}

#[test]
fn rotation_keeps_two_vaults_in_same_directory_independent() {
    // Two Vaults that happen to share a backup directory must rotate keyed
    // on their own basename, never touching the other's snapshots.
    let dir = tempdir().expect("tempdir");
    let vault_a = dir.path().join("vault-a.kdbx");
    let vault_b = dir.path().join("vault-b.kdbx");
    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let settings = BackupSettings {
        enabled: true,
        max_versions: 2,
        directory: None,
    };

    // 5 snapshots of vault A, 4 snapshots of vault B, interleaved so they
    // share the same .kdbx-backups/ directory.
    for i in 0..5u32 {
        std::fs::write(&vault_a, format!("a{i}").as_bytes()).expect("write a");
        snapshot(&vault_a, &settings)
            .expect("snapshot a ok")
            .expect("created a");
        if i < 4 {
            std::fs::write(&vault_b, format!("b{i}").as_bytes()).expect("write b");
            snapshot(&vault_b, &settings)
                .expect("snapshot b ok")
                .expect("created b");
        }
    }

    let files: Vec<String> = std::fs::read_dir(&backup_dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    let a_count = files
        .iter()
        .filter(|n| n.starts_with("vault-a.kdbx.backup."))
        .count();
    let b_count = files
        .iter()
        .filter(|n| n.starts_with("vault-b.kdbx.backup."))
        .count();

    assert_eq!(
        a_count, 2,
        "vault-a should have cap=2 snapshots, got {a_count}"
    );
    assert_eq!(
        b_count, 2,
        "vault-b should have cap=2 snapshots, got {b_count}"
    );
}

#[test]
fn rotation_preserves_foreign_files_in_backup_dir() {
    // The rotation glob must never touch files outside its pattern even
    // when they happen to live alongside our snapshots.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let settings = BackupSettings {
        enabled: true,
        max_versions: 1,
        directory: None,
    };

    // Seed one snapshot to force creation of .kdbx-backups/.
    std::fs::write(&vault_path, b"seed").expect("write source");
    snapshot(&vault_path, &settings)
        .expect("seed snapshot ok")
        .expect("created");

    // Plant a foreign-vault snapshot and unrelated files now that the dir exists.
    let foreign = backup_dir.join("other.kdbx.backup.20260101T000000.000Z.kdbx");
    let manual = backup_dir.join("vault.kdbx.backup.manual.20260101T000000.000Z.kdbx");
    let readme = backup_dir.join("README.txt");
    std::fs::write(&foreign, b"foreign").expect("write foreign");
    std::fs::write(&manual, b"manual").expect("write manual");
    std::fs::write(&readme, b"readme").expect("write readme");

    // Drive several rotations of our vault.
    for i in 0..4u32 {
        std::fs::write(&vault_path, format!("v{i}").as_bytes()).expect("write source");
        snapshot(&vault_path, &settings)
            .expect("snapshot ok")
            .expect("created");
    }

    assert!(
        foreign.exists(),
        "foreign-vault snapshot must not be deleted"
    );
    assert!(manual.exists(), "manual-marker file must not be deleted");
    assert!(readme.exists(), "unrelated file must not be deleted");

    let our_snapshots = std::fs::read_dir(&backup_dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("vault.kdbx.backup.") && !n.contains(".backup.manual.")
        })
        .count();
    assert_eq!(
        our_snapshots, 1,
        "cap=1 on our auto-snapshots, got {our_snapshots}"
    );
}

#[cfg(unix)]
#[test]
fn rotation_does_not_run_when_snapshot_fails() {
    use std::os::unix::fs::PermissionsExt;

    // Seed two pre-existing snapshots, then force the *next* snapshot to
    // fail by making the backup dir read-only. The existing two must remain.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let settings = BackupSettings {
        enabled: true,
        max_versions: 5,
        directory: None,
    };

    for i in 0..2u32 {
        std::fs::write(&vault_path, format!("seed-{i}").as_bytes()).expect("write source");
        snapshot(&vault_path, &settings)
            .expect("seed ok")
            .expect("seed created");
    }
    let pre_count = std::fs::read_dir(&backup_dir)
        .expect("read backup dir")
        .filter_map(Result::ok)
        .count();
    assert_eq!(pre_count, 2);

    // Lower max_versions so rotation *would* delete one if it ran. Force
    // the snapshot write to fail by making the source vault unreadable.
    // (Making the backup directory read-only here is futile: ensure_backup_dir
    // re-hardens it to 0700 on every call, restoring write permission.)
    let trim_settings = BackupSettings {
        enabled: true,
        max_versions: 1,
        directory: None,
    };
    std::fs::write(&vault_path, b"will-fail").expect("write source");
    let original_vault_perms = std::fs::metadata(&vault_path)
        .expect("vault meta")
        .permissions();
    std::fs::set_permissions(&vault_path, std::fs::Permissions::from_mode(0o000))
        .expect("set unreadable");

    let result = snapshot(&vault_path, &trim_settings);

    // Restore so tempdir cleanup works.
    std::fs::set_permissions(&vault_path, original_vault_perms).expect("restore");

    assert!(
        result.is_err(),
        "snapshot must fail when source vault cannot be read"
    );
    let our_snapshots = std::fs::read_dir(&backup_dir)
        .expect("read backup dir post")
        .filter_map(Result::ok)
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("vault.kdbx.backup.") && !n.contains(".backup.manual.")
        })
        .count();
    assert_eq!(
        our_snapshots, 2,
        "failed snapshot must leave existing backups untouched, got {our_snapshots}"
    );
}

#[test]
fn snapshot_creates_override_directory_if_missing() {
    // Cross-volume proxy: the override points at a parent tree that shares no
    // ancestor with the source Vault, and the leaf directory does not exist
    // yet. The snapshot must succeed and create the directory chain — exactly
    // what happens the first time a user points at a freshly-mounted drive.
    let vault_dir = tempdir().expect("tempdir vault");
    let override_root = tempdir().expect("tempdir override root");
    let nested_override = override_root.path().join("kdbx-snapshots").join("vault");
    assert!(!nested_override.exists(), "precondition: dir absent");

    let vault_path = vault_dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    let settings = BackupSettings {
        enabled: true,
        directory: Some(nested_override.to_string_lossy().into_owned()),
        ..BackupSettings::default()
    };

    let info = snapshot(&vault_path, &settings)
        .expect("snapshot ok")
        .expect("snapshot created");

    assert!(
        nested_override.is_dir(),
        "nested override dir should be created"
    );
    assert!(info.path.starts_with(&nested_override));
}

#[cfg(unix)]
#[test]
fn snapshot_fails_closed_when_override_is_unwritable() {
    // No eager validation at settings save time means an override that exists
    // but is read-only must surface as BackupFailed at the next save — same
    // shape as the MVP failure mode, so the UI/error pipeline stays uniform.
    use std::os::unix::fs::PermissionsExt;

    let vault_dir = tempdir().expect("tempdir vault");
    let override_root = tempdir().expect("tempdir override");
    let vault_path = vault_dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    // Make the override parent read-only so neither dir creation nor file
    // creation inside it can succeed.
    let mut perms = std::fs::metadata(override_root.path())
        .expect("meta")
        .permissions();
    let original = perms.mode();
    perms.set_mode(0o500);
    std::fs::set_permissions(override_root.path(), perms).expect("set ro");

    let override_dir = override_root.path().join("blocked");
    let settings = BackupSettings {
        enabled: true,
        directory: Some(override_dir.to_string_lossy().into_owned()),
        ..BackupSettings::default()
    };

    let result = snapshot(&vault_path, &settings);

    // Restore so tempdir cleanup works.
    let mut restore = std::fs::metadata(override_root.path())
        .expect("meta")
        .permissions();
    restore.set_mode(original);
    std::fs::set_permissions(override_root.path(), restore).expect("restore");

    match result {
        Err(BackupError::BackupFailed { path, .. }) => {
            assert!(
                path == override_dir || path.starts_with(override_dir.as_path()),
                "BackupFailed path should name the unwritable override, got {path:?}"
            );
        }
        other => panic!("expected BackupFailed, got {other:?}"),
    }
}

#[test]
fn rotation_runs_inside_override_directory() {
    // Rotation must apply to the override directory, not the default subdir.
    // Otherwise switching to an override would silently disable trimming and
    // the override volume would grow without bound.
    let vault_dir = tempdir().expect("tempdir vault");
    let override_dir = tempdir().expect("tempdir override");
    let vault_path = vault_dir.path().join("vault.kdbx");
    let settings = BackupSettings {
        enabled: true,
        max_versions: 3,
        directory: Some(override_dir.path().to_string_lossy().into_owned()),
    };

    for i in 0..5u32 {
        std::fs::write(&vault_path, format!("v{i}").as_bytes()).expect("write source");
        snapshot(&vault_path, &settings)
            .expect("snapshot ok")
            .expect("created");
    }

    let kept: Vec<_> = std::fs::read_dir(override_dir.path())
        .expect("read override")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .collect();
    assert_eq!(
        kept.len(),
        3,
        "after 5 saves with max_versions=3 inside override, exactly 3 remain"
    );

    let default_subdir = vault_dir.path().join(BACKUP_SUBDIR);
    assert!(
        !default_subdir.exists(),
        "default subdir must not be created or populated when override is in use"
    );
}

#[test]
fn snapshot_falls_back_to_default_subdir_when_override_cleared() {
    // Clearing the override (None) must resume the per-Vault sibling subdir
    // on the very next save — no app restart, no stale path resolution.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    let settings = BackupSettings {
        enabled: true,
        directory: None,
        ..BackupSettings::default()
    };
    let info = snapshot(&vault_path, &settings)
        .expect("snapshot ok")
        .expect("snapshot created");

    let default_subdir = dir.path().join(BACKUP_SUBDIR);
    assert!(
        info.path.starts_with(&default_subdir),
        "snapshot must land in default subdir when override is None, got {:?}",
        info.path
    );
}

#[test]
fn snapshot_uses_override_directory_when_set() {
    // With backups.directory = Some(absolute), snapshots must land inside
    // that path rather than the default sibling .kdbx-backups/.
    let vault_dir = tempdir().expect("tempdir vault");
    let override_dir = tempdir().expect("tempdir override");
    let vault_path = vault_dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"pre-image bytes").expect("write source");

    let settings = BackupSettings {
        enabled: true,
        directory: Some(override_dir.path().to_string_lossy().into_owned()),
        ..BackupSettings::default()
    };

    let info = snapshot(&vault_path, &settings)
        .expect("snapshot ok")
        .expect("snapshot created");

    assert!(
        info.path.starts_with(override_dir.path()),
        "snapshot must live under override dir, got {:?}",
        info.path
    );
    let default_subdir = vault_dir.path().join(BACKUP_SUBDIR);
    assert!(
        !default_subdir.exists(),
        "default sibling subdir must not be created when override is in use"
    );

    let bytes = std::fs::read(&info.path).expect("read snapshot");
    assert_eq!(bytes, b"pre-image bytes");
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
