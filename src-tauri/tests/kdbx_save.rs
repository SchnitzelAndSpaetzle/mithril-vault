#![allow(clippy::expect_used)]

use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use tempfile::tempdir;

#[path = "support/mod.rs"]
mod support;

use support::fixture_path;

#[test]
fn test_save_database() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("save-test.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path.to_string_lossy(), "savepass", "Save Test")
        .expect("Failed to create database");

    service.save().expect("Failed to save database");

    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "savepass")
        .expect("Failed to reopen after save");
}

#[test]
fn test_save_without_open_database() {
    let service = KdbxService::new();
    let result = service.save();
    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Save should fail when no database is open"
    );
}

#[test]
fn test_save_as_new_path() {
    let dir = tempdir().expect("Failed to create temp dir");
    let original_path = dir.path().join("original.kdbx");
    let new_path = dir.path().join("copy.kdbx");

    let service = KdbxService::new();
    service
        .create(&original_path.to_string_lossy(), "origpass", "Original")
        .expect("Failed to create database");

    service
        .save_as(&new_path.to_string_lossy(), None)
        .expect("Failed to save as");

    assert!(original_path.exists());
    assert!(new_path.exists());

    service.close().expect("Failed to close");
    service
        .open(&new_path.to_string_lossy(), "origpass")
        .expect("Failed to open new path");
}

#[test]
fn test_save_as_with_new_password() {
    let dir = tempdir().expect("Failed to create temp dir");
    let original_path = dir.path().join("original2.kdbx");
    let new_path = dir.path().join("newpass.kdbx");

    let service = KdbxService::new();
    service
        .create(&original_path.to_string_lossy(), "oldpass", "Test DB")
        .expect("Failed to create database");

    service
        .save_as(&new_path.to_string_lossy(), Some("newpass123"))
        .expect("Failed to save as with new password");

    service.close().expect("Failed to close");
    service
        .open(&new_path.to_string_lossy(), "newpass123")
        .expect("Failed to open with new password");
}

#[test]
fn test_save_preserves_keyfile_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-save-test.kdbx");

    let fixture_db = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "test123",
            &fixture_key.to_string_lossy(),
        )
        .expect("Failed to open with keyfile");

    service.save().expect("Failed to save");
    service.close().expect("Failed to close");

    let result = service.open(&db_path.to_string_lossy(), "test123");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Database should still require keyfile after save"
    );

    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "test123",
            &fixture_key.to_string_lossy(),
        )
        .expect("Should still open with keyfile after save");
}

#[test]
fn test_save_preserves_keyfile_only_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-only-save-test.kdbx");

    let fixture_db = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile-only fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &fixture_key.to_string_lossy())
        .expect("Failed to open with keyfile only");

    service.save().expect("Failed to save");
    service.close().expect("Failed to close");

    let result = service.open(&db_path.to_string_lossy(), "any_password");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Database should still require keyfile after save"
    );

    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &fixture_key.to_string_lossy())
        .expect("Should still open with keyfile after save");
}

#[test]
fn test_save_as_preserves_keyfile_only_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-only-save-as-test.kdbx");
    let new_path = dir.path().join("keyfile-only-copy.kdbx");

    let fixture_db = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile-only fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &fixture_key.to_string_lossy())
        .expect("Failed to open with keyfile only");

    service
        .save_as(&new_path.to_string_lossy(), None)
        .expect("Failed to save as");
    service.close().expect("Failed to close");

    let result = service.open(&new_path.to_string_lossy(), "any_password");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "New file should still require keyfile"
    );

    service
        .open_with_keyfile_only(&new_path.to_string_lossy(), &fixture_key.to_string_lossy())
        .expect("New file should open with keyfile");
}

#[test]
fn test_save_as_preserves_keyfile_plus_password_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-pw-save-as-test.kdbx");
    let new_path = dir.path().join("keyfile-pw-copy.kdbx");

    let fixture_db = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "test123",
            &fixture_key.to_string_lossy(),
        )
        .expect("Failed to open with password + keyfile");

    service
        .save_as(&new_path.to_string_lossy(), None)
        .expect("Failed to save as");
    service.close().expect("Failed to close");

    let result = service.open(&new_path.to_string_lossy(), "test123");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "New file should still require keyfile"
    );

    service
        .open_with_keyfile(
            &new_path.to_string_lossy(),
            "test123",
            &fixture_key.to_string_lossy(),
        )
        .expect("New file should open with password + keyfile");
}

#[test]
fn test_save_as_with_new_password_on_keyfile_database() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-newpw-test.kdbx");
    let new_path = dir.path().join("keyfile-newpw.kdbx");

    let fixture_db = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "test123",
            &fixture_key.to_string_lossy(),
        )
        .expect("Failed to open with password + keyfile");

    service
        .save_as(&new_path.to_string_lossy(), Some("newpassword456"))
        .expect("Failed to save as with new password");
    service.close().expect("Failed to close");

    let result = service.open_with_keyfile(
        &new_path.to_string_lossy(),
        "test123",
        &fixture_key.to_string_lossy(),
    );
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Old password should not work on new file"
    );

    service
        .open_with_keyfile(
            &new_path.to_string_lossy(),
            "newpassword456",
            &fixture_key.to_string_lossy(),
        )
        .expect("New password + keyfile should work");
}

#[test]
fn test_save_atomic_write_no_temp_file_remains() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("atomic-save.kdbx");
    let temp_path = dir.path().join(".atomic-save.kdbx.tmp");

    let service = KdbxService::new();
    service
        .create(&db_path.to_string_lossy(), "testpass", "Atomic Test")
        .expect("Failed to create database");

    service.save().expect("Failed to save database");

    assert!(
        !temp_path.exists(),
        "Temp file should not exist after successful save"
    );

    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpass")
        .expect("Failed to reopen after atomic save");
}

#[test]
fn test_save_clears_is_modified_flag() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("modified-flag.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path.to_string_lossy(), "testpass", "Modified Flag Test")
        .expect("Failed to create database");

    let info = service.get_info().expect("Failed to get info");
    assert!(
        !info.is_modified,
        "is_modified should be false after create"
    );

    service.save().expect("Failed to save");
    let info_after_save = service.get_info().expect("Failed to get info");
    assert!(
        !info_after_save.is_modified,
        "is_modified should be false after save"
    );
}

#[test]
fn test_save_as_creates_new_file_atomically() {
    let dir = tempdir().expect("Failed to create temp dir");
    let original_path = dir.path().join("original-atomic.kdbx");
    let new_path = dir.path().join("new-atomic.kdbx");
    let temp_path = dir.path().join(".new-atomic.kdbx.tmp");

    let service = KdbxService::new();
    service
        .create(&original_path.to_string_lossy(), "testpass", "Original")
        .expect("Failed to create database");

    service
        .save_as(&new_path.to_string_lossy(), None)
        .expect("Failed to save as");

    assert!(original_path.exists(), "Original file should exist");
    assert!(new_path.exists(), "New file should exist");

    assert!(!temp_path.exists(), "Temp file should not exist");

    service.close().expect("Failed to close");
    service
        .open(&new_path.to_string_lossy(), "testpass")
        .expect("Failed to open new file");
}

#[cfg(unix)]
#[test]
fn test_save_sets_secure_permissions_on_new_file() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("secure-perms.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path.to_string_lossy(), "testpass", "Secure Perms Test")
        .expect("Failed to create database");

    let metadata = std::fs::metadata(&db_path).expect("Should get metadata");
    let mode = metadata.permissions().mode() & 0o777;

    assert_eq!(
        mode, 0o600,
        "New database file should have 0600 permissions, got {mode:o}"
    );
}

#[cfg(unix)]
#[test]
fn test_save_preserves_existing_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("preserved-perms.kdbx");

    let service = KdbxService::new();
    service
        .create(
            &db_path.to_string_lossy(),
            "testpass",
            "Preserve Perms Test",
        )
        .expect("Failed to create database");

    let mut perms = std::fs::metadata(&db_path)
        .expect("Should get metadata")
        .permissions();
    perms.set_mode(0o640);
    std::fs::set_permissions(&db_path, perms).expect("Failed to set permissions");

    service.save().expect("Failed to save");

    let metadata_after = std::fs::metadata(&db_path).expect("Should get metadata after save");
    let mode_after = metadata_after.permissions().mode() & 0o777;

    assert_eq!(
        mode_after, 0o640,
        "Permissions should be preserved after save, got {mode_after:o}"
    );
}

#[test]
fn test_create_database_uses_atomic_write() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("atomic-create.kdbx");
    let temp_path = dir.path().join(".atomic-create.kdbx.tmp");

    let service = KdbxService::new();
    service
        .create(&db_path.to_string_lossy(), "testpass", "Atomic Create")
        .expect("Failed to create database");

    assert!(db_path.exists(), "Database file should exist");

    assert!(
        !temp_path.exists(),
        "Temp file should not exist after create"
    );

    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpass")
        .expect("Failed to reopen database");
}
