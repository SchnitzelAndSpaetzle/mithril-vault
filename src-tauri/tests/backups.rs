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
use mithril_vault_lib::services::kdbx::backups::{
    list_for, snapshot, snapshot_on_open, BackupKind, BACKUP_SUBDIR,
};
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
        on_open: false,
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
        on_open: false,
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
        on_open: false,
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
        on_open: false,
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
        on_open: false,
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
        on_open: false,
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
fn override_isolates_same_named_vaults_in_shared_directory() {
    // Two vaults with identical basenames (e.g. work/vault.kdbx and
    // personal/vault.kdbx) pointed at the same override directory must keep
    // independent rotation histories. Without per-source isolation, rotation
    // would key on basename alone and saving one vault would prune the
    // other's snapshots.
    let work_dir = tempdir().expect("tempdir work");
    let personal_dir = tempdir().expect("tempdir personal");
    let override_dir = tempdir().expect("tempdir override");

    let work_vault = work_dir.path().join("vault.kdbx");
    let personal_vault = personal_dir.path().join("vault.kdbx");

    let settings = BackupSettings {
        enabled: true,
        max_versions: 2,
        directory: Some(override_dir.path().to_string_lossy().into_owned()),
        on_open: false,
    };

    // Drive 4 saves of each vault, interleaved. With per-basename rotation
    // and cap=2, the second vault's saves would have pruned the first
    // vault's older snapshots — we'd end up with <4 surviving files.
    for i in 0..4u32 {
        std::fs::write(&work_vault, format!("w{i}").as_bytes()).expect("write work");
        snapshot(&work_vault, &settings)
            .expect("snapshot work ok")
            .expect("created work");
        std::fs::write(&personal_vault, format!("p{i}").as_bytes()).expect("write personal");
        snapshot(&personal_vault, &settings)
            .expect("snapshot personal ok")
            .expect("created personal");
    }

    let surviving_files: Vec<_> = walkdir_files(override_dir.path());
    assert_eq!(
        surviving_files.len(),
        4,
        "each vault should retain cap=2 snapshots; got {surviving_files:#?}"
    );
}

fn walkdir_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                out.push(path);
            }
        }
    }
    out
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
        on_open: false,
    };

    for i in 0..5u32 {
        std::fs::write(&vault_path, format!("v{i}").as_bytes()).expect("write source");
        snapshot(&vault_path, &settings)
            .expect("snapshot ok")
            .expect("created");
    }

    // Snapshots live in a per-vault subdir of the override; walk recursively
    // so the assertion stays correct regardless of isolation strategy.
    let kept = walkdir_files(override_dir.path());
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

fn on_open_enabled() -> BackupSettings {
    BackupSettings {
        enabled: true,
        on_open: true,
        ..BackupSettings::default()
    }
}

#[test]
fn snapshot_on_open_creates_snapshot_when_no_prior_exists() {
    // First open of a Vault on a fresh install: nothing in the backup dir
    // yet, so dedup has nothing to compare against and a snapshot is taken.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"pre-image bytes").expect("write source");

    let info = snapshot_on_open(&vault_path, &on_open_enabled())
        .expect("snapshot ok")
        .expect("snapshot created");

    assert!(info.path.exists(), "snapshot file should exist");
    let bytes = std::fs::read(&info.path).expect("read snapshot");
    assert_eq!(bytes, b"pre-image bytes");
}

#[test]
fn snapshot_on_open_dedups_when_source_unchanged() {
    // Lock-then-unlock scenario: the user re-opens the Vault, but nothing has
    // changed on disk between the two opens. The latest existing snapshot
    // already captures the exact bytes, so taking another one would just
    // burn a rotation slot for no information. The second call must skip.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"unchanged data").expect("write source");

    let first = snapshot_on_open(&vault_path, &on_open_enabled())
        .expect("first snapshot ok")
        .expect("first snapshot created");

    let second = snapshot_on_open(&vault_path, &on_open_enabled()).expect("second snapshot ok");
    assert!(
        second.is_none(),
        "second open with no changes must dedup (None), got {second:?}"
    );

    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let count = std::fs::read_dir(&backup_dir)
        .expect("read backup dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .count();
    assert_eq!(count, 1, "exactly one snapshot survives the dedup");
    assert!(first.path.exists(), "original snapshot still present");
}

#[test]
fn snapshot_on_open_takes_new_snapshot_after_source_changes() {
    // If the source was modified between two opens (the user saved from
    // another machine, or save-side took its pre-image snapshot in between),
    // dedup must NOT fire — there is new content worth preserving.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"v1 bytes").expect("write v1");

    let first = snapshot_on_open(&vault_path, &on_open_enabled())
        .expect("first ok")
        .expect("first created");

    // Mutate source so both size and mtime advance. Sleep a millisecond to
    // guarantee a coarse-mtime filesystem advances the mtime field.
    std::thread::sleep(std::time::Duration::from_millis(10));
    std::fs::write(&vault_path, b"v2 bytes that are longer").expect("write v2");

    let second = snapshot_on_open(&vault_path, &on_open_enabled())
        .expect("second ok")
        .expect("second should be created since source changed");

    assert_ne!(first.path, second.path, "second snapshot must be distinct");
    let bytes = std::fs::read(&second.path).expect("read");
    assert_eq!(bytes, b"v2 bytes that are longer");
}

#[test]
fn snapshot_on_open_skips_when_on_open_flag_is_false() {
    // Default-off per #193. Even when `enabled` is true the open-side hook
    // must do nothing unless the user has explicitly opted in.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    let settings = BackupSettings {
        enabled: true,
        on_open: false,
        ..BackupSettings::default()
    };
    let result = snapshot_on_open(&vault_path, &settings).expect("snapshot ok");
    assert!(result.is_none(), "on_open=false produces no snapshot");

    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    assert!(
        !backup_dir.exists(),
        "no backup dir should be created when on_open is off"
    );
}

#[test]
fn snapshot_on_open_streaming_compare_handles_payload_larger_than_buffer() {
    // KDBX vaults with attachments routinely run into the MB range — well
    // past `content_matches`'s 64 KiB read buffer. The streaming compare
    // must iterate correctly across multiple chunks: this exercises both
    // the "all identical" and "differs only after several chunks" paths.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");

    let mut v1 = vec![0u8; 256 * 1024];
    for (i, b) in v1.iter_mut().enumerate() {
        *b = u8::try_from(i % 251).unwrap_or(0);
    }
    std::fs::write(&vault_path, &v1).expect("write v1");

    snapshot_on_open(&vault_path, &on_open_enabled())
        .expect("first ok")
        .expect("first created");

    // Same bytes — dedup must hold across multi-chunk streaming.
    let dedup = snapshot_on_open(&vault_path, &on_open_enabled()).expect("dedup ok");
    assert!(dedup.is_none(), "identical multi-chunk content must dedup");

    // Flip a byte deep inside the file (past the first buffer) while
    // keeping size identical. Restore source mtime so size+mtime would
    // falsely match — only a byte-level compare can catch this.
    let original_mtime = std::fs::metadata(&vault_path)
        .expect("meta")
        .modified()
        .expect("mtime");
    let mut v2 = v1.clone();
    v2[200 * 1024] = v2[200 * 1024].wrapping_add(1);
    std::fs::write(&vault_path, &v2).expect("write v2");
    std::fs::OpenOptions::new()
        .write(true)
        .open(&vault_path)
        .expect("open")
        .set_modified(original_mtime)
        .expect("restore mtime");

    let after_change = snapshot_on_open(&vault_path, &on_open_enabled())
        .expect("after ok")
        .expect("changed multi-chunk content must produce a snapshot");
    let bytes = std::fs::read(&after_change.path).expect("read");
    assert_eq!(bytes, v2);
}

#[test]
fn snapshot_on_open_takes_snapshot_when_content_changes_with_same_size_and_mtime() {
    // KDBX writes encrypted blocks at fixed sizes; an external/synced save
    // that rewrites the encrypted payload can leave the file length and
    // even the mtime unchanged (some sync tools preserve mtime by design).
    // A metadata-only dedup would treat this as "no change" and miss the
    // snapshot. Content comparison must take a fresh snapshot.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"original-16-byte").expect("write v1");
    let original_mtime = std::fs::metadata(&vault_path)
        .expect("v1 meta")
        .modified()
        .expect("v1 mtime");

    snapshot_on_open(&vault_path, &on_open_enabled())
        .expect("first ok")
        .expect("first created");

    // Overwrite with same length but different bytes, then restore the
    // original mtime so neither len() nor modified() betrays the change.
    std::fs::write(&vault_path, b"REPLACED-16-byte").expect("write v2 same length");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&vault_path)
        .expect("open");
    file.set_modified(original_mtime)
        .expect("restore mtime to mask change");

    let second = snapshot_on_open(&vault_path, &on_open_enabled())
        .expect("second ok")
        .expect("changed content must produce a new snapshot");
    let bytes = std::fs::read(&second.path).expect("read");
    assert_eq!(bytes, b"REPLACED-16-byte");
}

#[test]
fn snapshot_on_open_dedup_survives_coarse_filesystem_mtime_rounding() {
    // Cross-filesystem regression test: when the backup override lives on a
    // coarser filesystem (FAT/exFAT round to 2 s; many SMB shares similar)
    // the snapshot's stamped mtime lands away from the source's. A
    // metadata-based dedup would fail and burn a rotation slot per open.
    // Content-based dedup must ignore the mtime drift entirely.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"unchanged data").expect("write source");

    // First snapshot.
    snapshot_on_open(&vault_path, &on_open_enabled())
        .expect("first ok")
        .expect("first created");

    // Simulate a coarse-mtime destination by rounding the latest snapshot's
    // mtime down to whole seconds. The source keeps its sub-second mtime.
    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let latest_snapshot = std::fs::read_dir(&backup_dir)
        .expect("read backup dir")
        .find_map(Result::ok)
        .expect("at least one snapshot")
        .path();
    let snapshot_mtime = std::fs::metadata(&latest_snapshot)
        .expect("snapshot meta")
        .modified()
        .expect("snapshot mtime");
    let since_epoch = snapshot_mtime
        .duration_since(std::time::UNIX_EPOCH)
        .expect("post-epoch");
    let rounded = std::time::UNIX_EPOCH + std::time::Duration::from_secs(since_epoch.as_secs());
    let snapshot_file = std::fs::OpenOptions::new()
        .write(true)
        .open(&latest_snapshot)
        .expect("open snapshot");
    snapshot_file
        .set_modified(rounded)
        .expect("round snapshot mtime");

    // Source is byte-identical; only the snapshot's filesystem-rounded mtime
    // differs by less than a second. Dedup must still fire.
    let second = snapshot_on_open(&vault_path, &on_open_enabled()).expect("second ok");
    assert!(
        second.is_none(),
        "sub-second mtime drift from coarse filesystem must not defeat dedup, got {second:?}"
    );
    let count = std::fs::read_dir(&backup_dir)
        .expect("read backup dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .count();
    assert_eq!(count, 1, "exactly one snapshot survives the tolerant dedup");
}

#[cfg(unix)]
#[test]
fn snapshot_on_open_rejects_symlinked_backup_dir_even_when_dedup_would_match() {
    // Defence in depth: the dedup short-circuit must not bypass the symlink
    // rejection that protects all snapshot writes. An attacker who can plant
    // a symlink at .kdbx-backups/ and a matching-metadata file inside its
    // target could otherwise suppress the open-side warning entirely.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"vault data").expect("write source");

    // Create a real directory elsewhere and seed it with a file that mimics
    // a valid snapshot of this Vault, with the same size as the source.
    let elsewhere = tempdir().expect("tempdir 2");
    let masquerade = elsewhere
        .path()
        .join("vault.kdbx.backup.20260101T000000.000Z.kdbx");
    std::fs::write(&masquerade, b"vault data").expect("write masquerade");

    // Then plant a symlink at the per-Vault sibling subdir so the open-side
    // dedup would, if it ran first, see the masquerading file as the latest
    // snapshot. Stamp the masquerade's mtime to match the source so dedup
    // would otherwise return Ok(None).
    let backup_link = dir.path().join(BACKUP_SUBDIR);
    std::os::unix::fs::symlink(elsewhere.path(), &backup_link).expect("symlink");
    let source_mtime = std::fs::metadata(&vault_path)
        .expect("source meta")
        .modified()
        .expect("mtime");
    let masquerade_file = std::fs::OpenOptions::new()
        .write(true)
        .open(&masquerade)
        .expect("open masquerade");
    masquerade_file
        .set_modified(source_mtime)
        .expect("stamp masquerade");

    let result = snapshot_on_open(&vault_path, &on_open_enabled());
    assert!(
        matches!(result, Err(BackupError::BackupFailed { .. })),
        "symlinked backup dir must abort the on-open hook, got {result:?}"
    );

    // Confirm no bytes were written into the symlink target.
    let extra: Vec<_> = std::fs::read_dir(elsewhere.path())
        .expect("read elsewhere")
        .filter_map(Result::ok)
        .filter(|e| e.file_name() != masquerade.file_name().unwrap_or_default())
        .collect();
    assert!(extra.is_empty(), "no new files via the symlink target");
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

#[test]
fn list_for_sorts_newest_first_by_parsed_timestamp() {
    // Three save-side snapshots, each with a distinct pre-image so a new
    // file lands each time. The listing must surface them with the newest
    // (most-recently-saved) at index 0.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let mut snapshot_paths = Vec::new();
    for i in 0..3u32 {
        std::fs::write(&vault_path, format!("v{i}").as_bytes()).expect("write source");
        let info = snapshot(&vault_path, &enabled())
            .expect("snapshot ok")
            .expect("created");
        snapshot_paths.push(info.path);
        // Ensure the next snapshot lands in a distinct millisecond so
        // ordering is unambiguous even on coarse-mtime filesystems.
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let listed = list_for(&vault_path, &enabled()).expect("list ok");
    assert_eq!(listed.len(), 3, "expected three listed backups");
    let paths: Vec<_> = listed.iter().map(|e| e.path.clone()).collect();
    // snapshot_paths is creation-order (oldest → newest); the listing must
    // be the reverse.
    let mut expected = snapshot_paths;
    expected.reverse();
    assert_eq!(paths, expected, "listing must be newest-first");
}

#[test]
fn list_for_includes_manual_snapshots() {
    // Manual snapshots live in the same directory as auto snapshots but use
    // a reserved `.backup.manual.` infix. The listing must include them and
    // classify the kind correctly.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    // Seed the directory by taking a real auto-snapshot first.
    let auto = snapshot(&vault_path, &enabled())
        .expect("snapshot ok")
        .expect("created");
    // Plant a manual-named file alongside it.
    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let manual_path = backup_dir.join("vault.kdbx.backup.manual.20260512T143045.123Z.kdbx");
    std::fs::write(&manual_path, b"manual bytes").expect("write manual");

    let listed = list_for(&vault_path, &enabled()).expect("list ok");
    assert_eq!(listed.len(), 2, "should list both auto and manual entries");

    let auto_entry = listed
        .iter()
        .find(|e| e.path == auto.path)
        .expect("auto entry present");
    assert_eq!(auto_entry.kind, BackupKind::Auto);

    let manual_entry = listed
        .iter()
        .find(|e| e.path == manual_path)
        .expect("manual entry present");
    assert_eq!(manual_entry.kind, BackupKind::Manual);
}

#[test]
fn list_for_skips_foreign_vault_snapshots() {
    // Two Vaults sharing a directory. Listing for vault.kdbx must return
    // only vault.kdbx's snapshots — never bleed across.
    let dir = tempdir().expect("tempdir");
    let vault_a = dir.path().join("vault.kdbx");
    let vault_b = dir.path().join("other.kdbx");
    std::fs::write(&vault_a, b"a").expect("write a");
    std::fs::write(&vault_b, b"b").expect("write b");
    let info_a = snapshot(&vault_a, &enabled())
        .expect("snap a")
        .expect("created a");
    let info_b = snapshot(&vault_b, &enabled())
        .expect("snap b")
        .expect("created b");

    let listed = list_for(&vault_a, &enabled()).expect("list a");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].path, info_a.path);

    let listed_b = list_for(&vault_b, &enabled()).expect("list b");
    assert_eq!(listed_b.len(), 1);
    assert_eq!(listed_b[0].path, info_b.path);
}

#[test]
fn list_for_returns_empty_when_backup_dir_missing() {
    // First-open scenario: nothing has ever been saved, the backup dir
    // does not exist. Listing must return an empty Vec, not error.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");

    let listed = list_for(&vault_path, &enabled()).expect("list ok");
    assert!(listed.is_empty(), "missing backup dir lists empty");
}

#[test]
fn list_for_returns_existing_auto_snapshot() {
    // Tracer bullet: after a single save-side snapshot, `list_for` must
    // surface that snapshot — same path, classified as auto.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"pre-image bytes").expect("write source");

    let info = snapshot(&vault_path, &enabled())
        .expect("snapshot ok")
        .expect("snapshot created");

    let listed = list_for(&vault_path, &enabled()).expect("list ok");
    assert_eq!(listed.len(), 1, "expected exactly one listed backup");
    let entry = &listed[0];
    assert_eq!(entry.path, info.path);
    assert_eq!(entry.kind, BackupKind::Auto);
    assert_eq!(entry.size_bytes, b"pre-image bytes".len() as u64);
    // Timestamp is ISO-8601 with the same UTC stamp encoded in the filename.
    assert!(
        entry.timestamp.starts_with("20") && entry.timestamp.ends_with('Z'),
        "timestamp should be ISO-8601 Zulu, got {:?}",
        entry.timestamp
    );
}
