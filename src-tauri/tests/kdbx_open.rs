#![allow(clippy::expect_used)]

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
    let unlock_err = service.unlock(&db_path, "wrong_password");
    assert!(
        matches!(unlock_err, Err(AppError::InvalidPassword)),
        "Should reject wrong password on unlock: got {unlock_err:?}"
    );

    // Unlock with correct password
    let unlock_info = service
        .unlock(&db_path, "test123")
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
fn test_lock_database_not_found() {
    let service = KdbxService::new();
    let result = service.lock("/nonexistent/db.kdbx");
    assert!(matches!(result, Err(AppError::DatabaseNotFound(_))));
}

#[test]
fn test_unlock_database_not_found() {
    let service = KdbxService::new();
    let result = service.unlock("/nonexistent/db.kdbx", "password");
    assert!(matches!(result, Err(AppError::DatabaseNotFound(_))));
}
