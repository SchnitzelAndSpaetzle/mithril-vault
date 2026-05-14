#![allow(clippy::expect_used)]

use mithril_vault_lib::domain::secure::SecureString;
use mithril_vault_lib::dto::entry::CreateEntryData;
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

#[path = "support/mod.rs"]
mod support;

use support::fixture_path;

/// Creates a temporary copy of a fixture file for isolated testing.
fn copy_fixture_to_temp(filename: &str) -> Option<(TempDir, PathBuf)> {
    let source = fixture_path(filename);
    if !source.exists() {
        return None;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let dest = temp_dir.path().join(filename);

    std::fs::copy(&source, &dest).expect("Failed to copy fixture");
    Some((temp_dir, dest))
}

/// Creates a temporary copy of keyfile fixtures for isolated testing.
fn copy_keyfile_fixtures_to_temp(
    db_filename: &str,
    key_filename: &str,
) -> Option<(TempDir, PathBuf, PathBuf)> {
    let db_source = fixture_path(db_filename);
    let key_source = fixture_path(key_filename);

    if !db_source.exists() || !key_source.exists() {
        return None;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_dest = temp_dir.path().join(db_filename);
    let key_dest = temp_dir.path().join(key_filename);

    std::fs::copy(&db_source, &db_dest).expect("Failed to copy database fixture");
    std::fs::copy(&key_source, &key_dest).expect("Failed to copy keyfile fixture");
    Some((temp_dir, db_dest, key_dest))
}

use mithril_vault_lib::commands::settings::BackupSettings;
use mithril_vault_lib::services::kdbx::backups::{BackupError, BACKUP_SUBDIR};

#[test]
fn snapshot_after_open_creates_snapshot_when_on_open_is_true() {
    // End-to-end through the service: an open() followed by
    // snapshot_after_open() with on_open=true must produce a backup file
    // alongside the source. This is the seam the command handler uses; if it
    // returns Ok(Some(_)) here, the open path can fire-and-forget the hook.
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("vault.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "test123", "Snapshot Test")
        .expect("create db");
    // close so we can hit the open path (which reads from disk).
    service.close(&db_path_str).expect("close db");

    service
        .set_backup_settings(BackupSettings {
            enabled: true,
            on_open: true,
            ..BackupSettings::default()
        })
        .expect("set backup settings");

    service
        .open(&db_path_str, "test123")
        .expect("open after close");

    let outcome = service
        .snapshot_after_open(&db_path_str)
        .expect("snapshot_after_open ok");
    assert!(
        outcome.is_some(),
        "first snapshot_after_open must produce a snapshot, got None"
    );

    let backup_dir = dir.path().join(BACKUP_SUBDIR);
    let snapshots: Vec<_> = std::fs::read_dir(&backup_dir)
        .expect("read backup dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .collect();
    assert_eq!(
        snapshots.len(),
        1,
        "exactly one backup file should exist post-open"
    );
}

#[test]
fn failed_password_open_never_produces_a_snapshot() {
    // Acceptance criterion: a failed password attempt must produce no
    // snapshot. The command short-circuits on the open error and never
    // reaches snapshot_after_open — verify the underlying invariant by
    // confirming no backup file appears even after a wrong-password open.
    let Some((temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let service = KdbxService::new();
    service
        .set_backup_settings(BackupSettings {
            enabled: true,
            on_open: true,
            ..BackupSettings::default()
        })
        .expect("set backup settings");

    let wrong = service.open(&path.to_string_lossy(), "wrong_password");
    assert!(
        matches!(wrong, Err(AppError::InvalidPassword)),
        "expected InvalidPassword, got {wrong:?}"
    );

    let backup_dir = temp_dir.path().join(BACKUP_SUBDIR);
    assert!(
        !backup_dir.exists(),
        "no backup directory should be created on a failed password attempt"
    );
}

#[cfg(unix)]
#[test]
fn snapshot_after_open_fails_open_when_backup_dir_is_unwritable() {
    // Fail-open semantics from #193: a backup failure during the open path
    // must NOT prevent the user from using the unlocked Vault. The service
    // surfaces the BackupError so the command can emit `backup-warning`, but
    // the database itself stays open and operational.
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("vault.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "test123", "Fail Open")
        .expect("create db");
    service.close(&db_path_str).expect("close db");

    // Point the override at a directory inside a read-only parent so neither
    // the dir nor any file inside it can be created.
    let blocked_root = tempdir().expect("tempdir blocked root");
    let mut perms = std::fs::metadata(blocked_root.path())
        .expect("meta")
        .permissions();
    let original_mode = perms.mode();
    perms.set_mode(0o500);
    std::fs::set_permissions(blocked_root.path(), perms).expect("set ro");
    let blocked_override = blocked_root.path().join("nope");

    service
        .set_backup_settings(BackupSettings {
            enabled: true,
            on_open: true,
            directory: Some(blocked_override.to_string_lossy().into_owned()),
            ..BackupSettings::default()
        })
        .expect("set backup settings");

    let open_result = service.open(&db_path_str, "test123");
    let snapshot_result = service.snapshot_after_open(&db_path_str);

    // Restore so tempdir cleanup works.
    let mut restore = std::fs::metadata(blocked_root.path())
        .expect("meta")
        .permissions();
    restore.set_mode(original_mode);
    std::fs::set_permissions(blocked_root.path(), restore).expect("restore");

    open_result.expect("open must succeed even when backup will fail");
    assert!(
        matches!(snapshot_result, Err(BackupError::BackupFailed { .. })),
        "snapshot_after_open should surface the backup failure, got {snapshot_result:?}"
    );

    // The database must still be open and usable.
    let info = service
        .get_info(&db_path_str)
        .expect("info after failed snapshot");
    assert!(!info.is_locked, "db must remain unlocked");
}

#[test]
fn test_open_kdbx4_with_password() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!(
            "Skipping test: fixture not found. \
             Create with KeePassXC using password 'test123'"
        );
        return;
    };

    let service = KdbxService::new();
    let info = service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX4 database");

    assert!(!info.name.is_empty(), "Root group should have a name");
}

#[test]
fn test_open_kdbx3_with_password() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx3-low-KDF.kdbx") else {
        eprintln!(
            "Skipping test: fixture not found. \
             Create with KeePassXC (KDBX 3.1 format) using password 'test123'"
        );
        return;
    };

    let service = KdbxService::new();
    let info = service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX3 database");

    assert!(!info.name.is_empty(), "Root group should have a name");
}

#[test]
fn test_open_kdbx3_returns_correct_version() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx3-low-KDF.kdbx") else {
        eprintln!("Skipping test: KDBX3 fixture not found");
        return;
    };

    let service = KdbxService::new();
    let info = service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX3 database");

    if !info.version.starts_with("KDBX 3.") {
        eprintln!(
            "Skipping test: fixture is {} format, not KDBX 3.x. \
             Recreate with KeePassXC using KDBX 3.1 format.",
            info.version
        );
        return;
    }

    assert_eq!(
        info.version, "KDBX 3.1",
        "KDBX3 database should report version 'KDBX 3.1'"
    );
}

#[test]
fn test_open_kdbx4_returns_correct_version() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: KDBX4 fixture not found");
        return;
    };

    let service = KdbxService::new();
    let info = service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX4 database");

    assert_eq!(
        info.version, "KDBX 4.0",
        "KDBX4 database should report version 'KDBX 4.0'"
    );
}

#[test]
fn test_kdbx3_invalid_password_rejection() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx3-low-KDF.kdbx") else {
        eprintln!("Skipping test: KDBX3 fixture not found");
        return;
    };

    let service = KdbxService::new();
    let result = service.open(&path.to_string_lossy(), "wrong_password");

    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "KDBX3 should reject invalid password"
    );
}

#[test]
fn test_create_database_returns_kdbx4_version() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("version-test.kdbx");

    let service = KdbxService::new();
    let info = service
        .create(&db_path.to_string_lossy(), "testpass", "Version Test")
        .expect("Failed to create database");

    assert_eq!(info.version, "KDBX 4.0", "New databases should be KDBX 4.0");
}

#[test]
fn test_get_info_returns_version() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: KDBX4 fixture not found");
        return;
    };

    let db_path = path.to_string_lossy().to_string();
    let service = KdbxService::new();
    service
        .open(&db_path, "test123")
        .expect("Failed to open database");

    let info = service
        .get_info(&db_path)
        .expect("Failed to get database info");

    assert_eq!(info.version, "KDBX 4.0", "get_info() should return version");
}

#[test]
fn test_open_with_keyfile() {
    let Some((_temp_dir, db_path, key_path)) =
        copy_keyfile_fixtures_to_temp("test-keyfile-kdbx4-low-KDF.kdbx", "test-keyfile.keyx")
    else {
        eprintln!(
            "Skipping test: fixtures not found. \
             Create database with password 'test123' and keyfile"
        );
        return;
    };

    let service = KdbxService::new();
    let info = service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "test123",
            &key_path.to_string_lossy(),
        )
        .expect("Failed to open database with keyfile");
    assert!(!info.name.is_empty());
}

#[test]
fn test_invalid_password() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let service = KdbxService::new();
    let result = service.open(&path.to_string_lossy(), "wrong_password");

    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Should fail with invalid password"
    );
}

#[test]
fn test_file_not_found() {
    let path = fixture_path("nonexistent.kdbx");
    let service = KdbxService::new();
    let result = service.open(&path.to_string_lossy(), "test123");

    assert!(
        matches!(result, Err(AppError::InvalidPath(_))),
        "Should fail when file doesn't exist"
    );
}

#[test]
fn test_open_twice_and_close() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let db_path = path.to_string_lossy().to_string();
    let service = KdbxService::new();
    service
        .open(&db_path, "test123")
        .expect("Failed to open database");

    let result = service.open(&db_path, "test123");
    assert!(
        matches!(result, Err(AppError::DatabaseAlreadyOpen(_))),
        "Should not allow opening twice"
    );

    service.close(&db_path).expect("Failed to close database");
    let info_after_close = service.get_info(&db_path);
    assert!(
        matches!(info_after_close, Err(AppError::DatabaseNotFound(_))),
        "Should not return info after close"
    );
}

#[test]
fn test_close_without_open() {
    let service = KdbxService::new();
    let result = service.close("/nonexistent/db.kdbx");
    assert!(
        matches!(result, Err(AppError::DatabaseNotFound(_))),
        "Should error when closing without an open database"
    );
}

#[test]
fn test_open_with_keyfile_only_success() {
    let Some((_temp_dir, db_path, key_path)) =
        copy_keyfile_fixtures_to_temp("test-keyfile-only-kdbx4-low-KDF.kdbx", "test-keyfile.keyx")
    else {
        eprintln!(
            "Skipping test: keyfile-only fixtures not found. \
             Create database with keyfile-only authentication using test-keyfile.keyx"
        );
        return;
    };

    let service = KdbxService::new();
    let info = service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &key_path.to_string_lossy())
        .expect("Failed to open database with keyfile only");

    assert!(!info.name.is_empty(), "Root group should have a name");
    assert_eq!(info.version, "KDBX 4.0");
}

#[test]
fn test_open_with_keyfile_only_wrong_keyfile() {
    let Some((_temp_dir, db_path)) = copy_fixture_to_temp("test-keyfile-only-kdbx4-low-KDF.kdbx")
    else {
        eprintln!("Skipping test: keyfile-only fixture not found");
        return;
    };

    let dir = tempdir().expect("Failed to create temp dir");
    let fake_keyfile = dir.path().join("wrong-keyfile.keyx");
    std::fs::write(&fake_keyfile, b"wrong keyfile content").expect("Failed to write fake keyfile");

    let service = KdbxService::new();
    let result =
        service.open_with_keyfile_only(&db_path.to_string_lossy(), &fake_keyfile.to_string_lossy());

    assert!(
        matches!(
            result,
            Err(AppError::InvalidPassword | AppError::KeyfileInvalid)
        ),
        "Should fail with wrong keyfile: got {result:?}"
    );
}

#[test]
fn test_keyfile_not_found_error() {
    let Some((_temp_dir, db_path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let service = KdbxService::new();
    let result = service.open_with_keyfile_only(
        &db_path.to_string_lossy(),
        "/nonexistent/path/to/keyfile.keyx",
    );

    assert!(
        matches!(result, Err(AppError::KeyfileNotFound)),
        "Should fail with keyfile not found error: got {result:?}"
    );
}

#[test]
fn test_keyfile_not_found_for_password_plus_keyfile() {
    let Some((_temp_dir, db_path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let service = KdbxService::new();
    let result = service.open_with_keyfile(
        &db_path.to_string_lossy(),
        "test123",
        "/nonexistent/path/to/keyfile.keyx",
    );

    assert!(
        matches!(result, Err(AppError::InvalidPath(_))),
        "Should fail when keyfile path doesn't exist: got {result:?}"
    );
}

#[test]
fn test_lock_and_unlock() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let db_path = path.to_string_lossy().to_string();
    let service = KdbxService::new();
    let open_info = service.open(&db_path, "test123").expect("Failed to open");
    assert!(!open_info.is_locked);

    // Lock the database
    let lock_info = service.lock(&db_path).expect("Failed to lock");
    assert!(lock_info.is_locked);
    assert_eq!(lock_info.name, open_info.name);
    assert_eq!(lock_info.root_group_id, open_info.root_group_id);

    // Operations should fail while locked
    let list_result = service.list_entries(&db_path, None);
    assert!(
        matches!(list_result, Err(AppError::DatabaseLocked(_))),
        "Should not allow listing entries when locked: got {list_result:?}"
    );

    // Unlock with wrong password should fail
    let unlock_err = service.unlock(&db_path, Some("wrong_password"));
    assert!(
        matches!(unlock_err, Err(AppError::InvalidPassword)),
        "Should reject wrong password on unlock: got {unlock_err:?}"
    );

    // Unlock with correct password
    let unlock_info = service
        .unlock(&db_path, Some("test123"))
        .expect("Failed to unlock");
    assert!(!unlock_info.is_locked);
    assert_eq!(unlock_info.name, open_info.name);

    // Operations should work again
    let entries = service
        .list_entries(&db_path, None)
        .expect("Failed to list entries after unlock");
    let _ = entries; // Just verify it doesn't error
}

#[test]
fn test_lock_all() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path1 = dir.path().join("db1.kdbx");
    let db_path2 = dir.path().join("db2.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path1.to_string_lossy(), "pass1", "DB1")
        .expect("Failed to create db1");
    service
        .create(&db_path2.to_string_lossy(), "pass2", "DB2")
        .expect("Failed to create db2");

    let locked = service.lock_all().expect("Failed to lock all");
    assert_eq!(locked.len(), 2);

    // Both should be locked
    let info1 = service
        .get_info(&db_path1.to_string_lossy())
        .expect("Failed to get info");
    let info2 = service
        .get_info(&db_path2.to_string_lossy())
        .expect("Failed to get info");
    assert!(info1.is_locked);
    assert!(info2.is_locked);

    // lock_all on already locked databases returns empty list
    let locked_again = service.lock_all().expect("Failed to lock all again");
    assert!(locked_again.is_empty());
}

#[test]
fn test_lock_all_skips_modified_databases() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path1 = dir.path().join("dirty.kdbx");
    let db_path2 = dir.path().join("clean.kdbx");
    let db_path1_str = db_path1.to_string_lossy().to_string();
    let db_path2_str = db_path2.to_string_lossy().to_string();

    let service = KdbxService::new();
    let info1 = service
        .create(&db_path1_str, "pass1", "Dirty DB")
        .expect("Failed to create dirty db");
    service
        .create(&db_path2_str, "pass2", "Clean DB")
        .expect("Failed to create clean db");

    service
        .create_entry(
            &db_path1_str,
            &info1.root_group_id,
            CreateEntryData {
                title: "Unsaved entry".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
            },
        )
        .expect("Failed to create entry");

    let locked = service.lock_all().expect("Failed to lock all");
    assert_eq!(locked, vec![db_path2_str.clone()]);

    let dirty_info = service
        .get_info(&db_path1_str)
        .expect("Failed to get dirty db info");
    assert!(!dirty_info.is_locked);
    assert!(dirty_info.is_modified);

    let clean_info = service
        .get_info(&db_path2_str)
        .expect("Failed to get clean db info");
    assert!(clean_info.is_locked);
    assert!(!clean_info.is_modified);
}

#[test]
fn test_lock_database_not_found() {
    let service = KdbxService::new();
    let result = service.lock("/nonexistent/db.kdbx");
    assert!(matches!(result, Err(AppError::DatabaseNotFound(_))));
}

#[test]
fn test_unlock_database_not_found() {
    let service = KdbxService::new();
    let result = service.unlock("/nonexistent/db.kdbx", Some("password"));
    assert!(matches!(result, Err(AppError::DatabaseNotFound(_))));
}

#[test]
fn test_unlock_keyfile_only_database_without_password() {
    let Some((_temp_dir, db_path, key_path)) =
        copy_keyfile_fixtures_to_temp("test-keyfile-only-kdbx4-low-KDF.kdbx", "test-keyfile.keyx")
    else {
        eprintln!("Skipping test: keyfile-only fixtures not found");
        return;
    };

    let db_path = db_path.to_string_lossy().to_string();
    let key_path = key_path.to_string_lossy().to_string();
    let service = KdbxService::new();

    service
        .open_with_keyfile_only(&db_path, &key_path)
        .expect("Failed to open with keyfile only");

    let lock_info = service.lock(&db_path).expect("Failed to lock");
    assert!(lock_info.is_locked);

    let unlock_info = service
        .unlock(&db_path, None)
        .expect("Failed to unlock keyfile-only database");
    assert!(!unlock_info.is_locked);

    let _ = service
        .list_entries(&db_path, None)
        .expect("Failed to list entries after unlock");
}
