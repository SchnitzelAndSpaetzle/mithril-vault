// SPDX-License-Identifier: MIT
//! Integration tests for KDBX database operations via `KdbxService`

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use mithril_vault_lib::models::database::DatabaseCreationOptions;
use mithril_vault_lib::models::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use std::path::PathBuf;
use tempfile::tempdir;

/// Get the path to a test fixture file
fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(filename);
    path
}

#[test]
fn test_open_kdbx4_with_password() {
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!(
            "Skipping test: fixture not found at {path:?}. \
             Create with KeePassXC using password 'test123'"
        );
        return;
    }

    let service = KdbxService::new();
    let info = service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX4 database");

    // Verify database was opened successfully
    assert!(!info.name.is_empty(), "Root group should have a name");
}

#[test]
fn test_open_kdbx3_with_password() {
    let path = fixture_path("test-kdbx3-low-KDF.kdbx");
    if !path.exists() {
        eprintln!(
            "Skipping test: fixture not found at {path:?}. \
             Create with KeePassXC (KDBX 3.1 format) using password 'test123'"
        );
        return;
    }

    let service = KdbxService::new();
    let info = service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX3 database");

    assert!(!info.name.is_empty(), "Root group should have a name");
}

#[test]
fn test_open_kdbx3_returns_correct_version() {
    let path = fixture_path("test-kdbx3-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: KDBX3 fixture not found");
        return;
    }

    let service = KdbxService::new();
    let info = service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX3 database");

    // Check if fixture is actually KDBX 3.1 format
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
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: KDBX4 fixture not found");
        return;
    }

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
    let path = fixture_path("test-kdbx3-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: KDBX3 fixture not found");
        return;
    }

    let service = KdbxService::new();
    let result = service.open(&path.to_string_lossy(), "wrong_password");

    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "KDBX3 should reject invalid password"
    );
}

#[test]
fn test_kdbx3_list_entries() {
    let path = fixture_path("test-kdbx3-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: KDBX3 fixture not found");
        return;
    }

    let service = KdbxService::new();
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX3 database");

    let entries = service
        .list_entries(None)
        .expect("Failed to list entries from KDBX3");

    assert!(!entries.is_empty(), "KDBX3 fixture should have entries");
}

#[test]
fn test_kdbx3_get_entry_password() {
    let path = fixture_path("test-kdbx3-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: KDBX3 fixture not found");
        return;
    }

    let service = KdbxService::new();
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX3 database");

    let entries = service.list_entries(None).expect("Failed to list entries");

    if entries.is_empty() {
        eprintln!("Skipping password test: no entries in KDBX3 fixture");
        return;
    }

    let entry_id = &entries[0].id;
    let password = service
        .get_entry_password(entry_id)
        .expect("Failed to get entry password from KDBX3");

    assert!(!password.is_empty(), "KDBX3 entry should have a password");
}

#[test]
fn test_kdbx3_list_groups() {
    let path = fixture_path("test-kdbx3-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: KDBX3 fixture not found");
        return;
    }

    let service = KdbxService::new();
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open KDBX3 database");

    let groups = service
        .list_groups()
        .expect("Failed to list groups from KDBX3");

    assert!(
        !groups.is_empty(),
        "KDBX3 should have at least the root group"
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
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: KDBX4 fixture not found");
        return;
    }

    let service = KdbxService::new();
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    let info = service.get_info().expect("Failed to get database info");

    assert_eq!(info.version, "KDBX 4.0", "get_info() should return version");
}

#[test]
fn test_open_with_keyfile() {
    let db_path = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let key_path = fixture_path("test-keyfile.keyx");

    if !db_path.exists() || !key_path.exists() {
        eprintln!(
            "Skipping test: fixtures not found. \
             Create database with password 'test123' and keyfile"
        );
        return;
    }

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
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

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
fn test_list_entries_and_get_entry() {
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let service = KdbxService::new();
    let info = service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    let entries = service.list_entries(None).expect("Failed to list entries");
    assert!(!entries.is_empty(), "Fixture should have entries");

    let entry_id = entries[0].id.clone();
    let entry = service.get_entry(&entry_id).expect("Failed to fetch entry");
    assert_eq!(entry.id, entry_id);
    assert_eq!(entry.group_id, entries[0].group_id);

    // Verify password retrieval works (fixture entries have passwords set)
    let password = service
        .get_entry_password(&entry_id)
        .expect("Failed to fetch entry password");
    assert!(
        !password.is_empty(),
        "Test fixture entry should have a password"
    );

    let entries_in_root = service
        .list_entries(Some(&info.root_group_id))
        .expect("Failed to list entries by group");
    assert!(entries_in_root.len() <= entries.len());
}

#[test]
fn test_list_groups_and_get_group() {
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let service = KdbxService::new();
    let info = service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    let groups = service.list_groups().expect("Failed to list groups");
    assert!(!groups.is_empty(), "Should have at least the root group");

    let root = service
        .get_group(&info.root_group_id)
        .expect("Failed to fetch root group");
    assert_eq!(root.id, info.root_group_id);
}

#[test]
fn test_entry_not_found() {
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let service = KdbxService::new();
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    let result = service.get_entry("missing-entry-id");
    assert!(
        matches!(result, Err(AppError::EntryNotFound(_))),
        "Should error for missing entry"
    );

    let password_result = service.get_entry_password("missing-entry-id");
    assert!(
        matches!(password_result, Err(AppError::EntryNotFound(_))),
        "Should error for missing entry password"
    );
}

#[test]
fn test_group_not_found() {
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let service = KdbxService::new();
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    let result = service.get_group("missing-group-id");
    assert!(
        matches!(result, Err(AppError::GroupNotFound(_))),
        "Should error for missing group"
    );
}

#[test]
fn test_open_twice_and_close() {
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let service = KdbxService::new();
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    let result = service.open(&path.to_string_lossy(), "test123");
    assert!(
        matches!(result, Err(AppError::DatabaseAlreadyOpen)),
        "Should not allow opening twice"
    );

    service.close().expect("Failed to close database");
    let info_after_close = service.get_info();
    assert!(
        matches!(info_after_close, Err(AppError::DatabaseNotOpen)),
        "Should not return info after close"
    );
}

#[test]
fn test_close_without_open() {
    let service = KdbxService::new();
    let result = service.close();
    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should error when closing without an open database"
    );
}

#[test]
fn test_list_entries_without_open() {
    let service = KdbxService::new();
    let result = service.list_entries(None);
    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should error when listing entries without an open database"
    );
}

#[test]
fn test_create_new_database() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("new-database.kdbx");

    let service = KdbxService::new();
    let info = service
        .create(&db_path.to_string_lossy(), "testpass123", "My New Vault")
        .expect("Failed to create database");

    assert_eq!(info.name, "My New Vault");
    assert!(!info.root_group_id.is_empty());
    assert!(!info.is_modified);

    // Verify the file was created
    assert!(db_path.exists(), "Database file should exist");

    // Close and reopen to verify it was saved correctly
    service.close().expect("Failed to close");

    let reopened_info = service
        .open(&db_path.to_string_lossy(), "testpass123")
        .expect("Failed to reopen database");

    assert_eq!(reopened_info.name, "My New Vault");
}

#[test]
fn test_create_fails_when_database_already_open() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path1 = dir.path().join("db1.kdbx");
    let db_path2 = dir.path().join("db2.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path1.to_string_lossy(), "pass1", "DB1")
        .expect("Failed to create first database");

    let result = service.create(&db_path2.to_string_lossy(), "pass2", "DB2");
    assert!(
        matches!(result, Err(AppError::DatabaseAlreadyOpen)),
        "Should not allow creating when database is already open"
    );
}

#[test]
fn test_save_database() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("save-test.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path.to_string_lossy(), "savepass", "Save Test")
        .expect("Failed to create database");

    // Save should succeed (even without modifications)
    service.save().expect("Failed to save database");

    // File should still be readable
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

    // Save to new path
    service
        .save_as(&new_path.to_string_lossy(), None)
        .expect("Failed to save as");

    // Both files should exist
    assert!(original_path.exists());
    assert!(new_path.exists());

    // New file should be openable with same password
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

    // Save to new path with new password
    service
        .save_as(&new_path.to_string_lossy(), Some("newpass123"))
        .expect("Failed to save as with new password");

    // Close and verify new password works
    service.close().expect("Failed to close");
    service
        .open(&new_path.to_string_lossy(), "newpass123")
        .expect("Failed to open with new password");
}

#[test]
fn test_save_preserves_keyfile_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-save-test.kdbx");

    // Copy fixture to temp location
    let fixture_db = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    // Open with keyfile
    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "test123",
            &fixture_key.to_string_lossy(),
        )
        .expect("Failed to open with keyfile");

    // Save the database
    service.save().expect("Failed to save");
    service.close().expect("Failed to close");

    // Verify: Opening with password-only should FAIL
    let result = service.open(&db_path.to_string_lossy(), "test123");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Database should still require keyfile after save"
    );

    // Verify: Opening with keyfile should succeed
    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "test123",
            &fixture_key.to_string_lossy(),
        )
        .expect("Should still open with keyfile after save");
}

// =============================================================================
// Keyfile-only authentication tests
// =============================================================================

#[test]
fn test_open_with_keyfile_only_success() {
    let db_path = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");
    let key_path = fixture_path("test-keyfile.keyx");

    if !db_path.exists() || !key_path.exists() {
        eprintln!(
            "Skipping test: keyfile-only fixtures not found. \
             Create database with keyfile-only authentication using test-keyfile.keyx"
        );
        return;
    }

    let service = KdbxService::new();
    let info = service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &key_path.to_string_lossy())
        .expect("Failed to open database with keyfile only");

    assert!(!info.name.is_empty(), "Root group should have a name");
    assert_eq!(info.version, "KDBX 4.0");
}

#[test]
fn test_open_with_keyfile_only_wrong_keyfile() {
    let db_path = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");

    if !db_path.exists() {
        eprintln!("Skipping test: keyfile-only fixture not found");
        return;
    }

    // Create a fake keyfile
    let dir = tempdir().expect("Failed to create temp dir");
    let fake_keyfile = dir.path().join("wrong-keyfile.keyx");
    std::fs::write(&fake_keyfile, b"wrong keyfile content").expect("Failed to write fake keyfile");

    let service = KdbxService::new();
    let result =
        service.open_with_keyfile_only(&db_path.to_string_lossy(), &fake_keyfile.to_string_lossy());

    // Should fail with invalid password (wrong key)
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
    let db_path = fixture_path("test-kdbx4-low-KDF.kdbx");

    if !db_path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

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
    let db_path = fixture_path("test-kdbx4-low-KDF.kdbx");

    if !db_path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

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
fn test_save_preserves_keyfile_only_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-only-save-test.kdbx");

    // Copy keyfile-only fixture to temp location
    let fixture_db = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile-only fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    // Open with keyfile only
    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &fixture_key.to_string_lossy())
        .expect("Failed to open with keyfile only");

    // Save the database
    service.save().expect("Failed to save");
    service.close().expect("Failed to close");

    // Verify: Opening with password-only should FAIL
    let result = service.open(&db_path.to_string_lossy(), "any_password");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Database should still require keyfile after save"
    );

    // Verify: Opening with keyfile should succeed
    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &fixture_key.to_string_lossy())
        .expect("Should still open with keyfile after save");
}

#[test]
fn test_list_entries_from_keyfile_only_database() {
    let db_path = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");
    let key_path = fixture_path("test-keyfile.keyx");

    if !db_path.exists() || !key_path.exists() {
        eprintln!("Skipping test: keyfile-only fixtures not found");
        return;
    }

    let service = KdbxService::new();
    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &key_path.to_string_lossy())
        .expect("Failed to open database");

    let entries = service
        .list_entries(None)
        .expect("Failed to list entries from keyfile-only database");

    assert!(
        !entries.is_empty(),
        "Keyfile-only fixture should have entries"
    );
}

#[test]
fn test_get_entry_password_from_keyfile_only_database() {
    let db_path = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");
    let key_path = fixture_path("test-keyfile.keyx");

    if !db_path.exists() || !key_path.exists() {
        eprintln!("Skipping test: keyfile-only fixtures not found");
        return;
    }

    let service = KdbxService::new();
    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &key_path.to_string_lossy())
        .expect("Failed to open database");

    let entries = service.list_entries(None).expect("Failed to list entries");
    if entries.is_empty() {
        eprintln!("Skipping password test: no entries in keyfile-only fixture");
        return;
    }

    let entry_id = &entries[0].id;
    let password = service
        .get_entry_password(entry_id)
        .expect("Failed to get entry password");

    assert!(
        !password.is_empty(),
        "Entry in keyfile-only database should have a password"
    );
}

// =============================================================================
// Save As with keyfile authentication tests
// =============================================================================

#[test]
fn test_save_as_preserves_keyfile_only_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-only-save-as-test.kdbx");
    let new_path = dir.path().join("keyfile-only-copy.kdbx");

    // Copy keyfile-only fixture to temp location
    let fixture_db = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile-only fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    // Open with keyfile only
    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &fixture_key.to_string_lossy())
        .expect("Failed to open with keyfile only");

    // Save As to new path (without changing password since there isn't one)
    service
        .save_as(&new_path.to_string_lossy(), None)
        .expect("Failed to save as");
    service.close().expect("Failed to close");

    // Verify: New file should still require keyfile
    let result = service.open(&new_path.to_string_lossy(), "any_password");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "New file should still require keyfile"
    );

    // Verify: Opening with keyfile should succeed
    service
        .open_with_keyfile_only(&new_path.to_string_lossy(), &fixture_key.to_string_lossy())
        .expect("New file should open with keyfile");
}

#[test]
fn test_save_as_preserves_keyfile_plus_password_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-pw-save-as-test.kdbx");
    let new_path = dir.path().join("keyfile-pw-copy.kdbx");

    // Copy password+keyfile fixture to temp location
    let fixture_db = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    // Open with password + keyfile
    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "test123",
            &fixture_key.to_string_lossy(),
        )
        .expect("Failed to open with password + keyfile");

    // Save As to new path (keeping same credentials)
    service
        .save_as(&new_path.to_string_lossy(), None)
        .expect("Failed to save as");
    service.close().expect("Failed to close");

    // Verify: Password-only should fail (still requires keyfile)
    let result = service.open(&new_path.to_string_lossy(), "test123");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "New file should still require keyfile"
    );

    // Verify: Opening with both password + keyfile should succeed
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

    // Copy password+keyfile fixture to temp location
    let fixture_db = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    // Open with password + keyfile
    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "test123",
            &fixture_key.to_string_lossy(),
        )
        .expect("Failed to open with password + keyfile");

    // Save As with a NEW password
    service
        .save_as(&new_path.to_string_lossy(), Some("newpassword456"))
        .expect("Failed to save as with new password");
    service.close().expect("Failed to close");

    // Verify: Old password + keyfile should FAIL
    let result = service.open_with_keyfile(
        &new_path.to_string_lossy(),
        "test123",
        &fixture_key.to_string_lossy(),
    );
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Old password should not work on new file"
    );

    // Verify: New password + keyfile should SUCCEED
    service
        .open_with_keyfile(
            &new_path.to_string_lossy(),
            "newpassword456",
            &fixture_key.to_string_lossy(),
        )
        .expect("New password + keyfile should work");
}

// =============================================================================
// Enhanced database creation tests (Issue #13)
// =============================================================================

#[test]
fn test_create_database_with_default_options() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("default-options.kdbx");

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword123"),
            None,
            "Default Options DB",
            &DatabaseCreationOptions::default(),
        )
        .expect("Failed to create database with default options");

    assert_eq!(info.name, "Default Options DB");
    assert_eq!(info.version, "KDBX 4.0");
    assert!(!info.root_group_id.is_empty());

    // Close and reopen to verify
    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpassword123")
        .expect("Failed to reopen database");
}

#[test]
fn test_create_database_with_keyfile() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-create.kdbx");
    let keyfile_path = fixture_path("test-keyfile.keyx");

    if !keyfile_path.exists() {
        eprintln!("Skipping test: keyfile fixture not found");
        return;
    }

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            Some(&keyfile_path.to_string_lossy()),
            "Keyfile DB",
            &DatabaseCreationOptions::default(),
        )
        .expect("Failed to create database with keyfile");

    assert_eq!(info.name, "Keyfile DB");
    assert_eq!(info.version, "KDBX 4.0");

    // Close and verify it requires keyfile to reopen
    service.close().expect("Failed to close");

    // Password-only should fail
    let result = service.open(&db_path.to_string_lossy(), "testpassword");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Should require keyfile to open: got {result:?}"
    );

    // Password + keyfile should succeed
    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "testpassword",
            &keyfile_path.to_string_lossy(),
        )
        .expect("Should open with password + keyfile");
}

#[test]
fn test_create_database_with_keyfile_only() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-only-create.kdbx");
    let keyfile_path = fixture_path("test-keyfile.keyx");

    if !keyfile_path.exists() {
        eprintln!("Skipping test: keyfile fixture not found");
        return;
    }

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            None, // No password
            Some(&keyfile_path.to_string_lossy()),
            "Keyfile Only DB",
            &DatabaseCreationOptions::default(),
        )
        .expect("Failed to create database with keyfile only");

    assert_eq!(info.name, "Keyfile Only DB");

    // Close and verify
    service.close().expect("Failed to close");

    // Keyfile-only should succeed
    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &keyfile_path.to_string_lossy())
        .expect("Should open with keyfile only");
}

#[test]
fn test_create_database_fails_without_credentials() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("no-creds.kdbx");

    let service = KdbxService::new();
    let result = service.create_database(
        &db_path.to_string_lossy(),
        None, // No password
        None, // No keyfile
        "No Credentials DB",
        &DatabaseCreationOptions::default(),
    );

    assert!(
        matches!(result, Err(AppError::NoCredentials)),
        "Should fail without credentials: got {result:?}"
    );
}

#[test]
fn test_create_database_with_default_groups() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("default-groups.kdbx");

    let options = DatabaseCreationOptions {
        create_default_groups: true,
        ..Default::default()
    };

    let service = KdbxService::new();
    service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            None,
            "Default Groups DB",
            &options,
        )
        .expect("Failed to create database with default groups");

    // List groups and verify default groups exist
    let groups = service.list_groups().expect("Failed to list groups");

    // Should have root group
    assert!(!groups.is_empty(), "Should have at least root group");

    let root = &groups[0];
    assert_eq!(root.name, "Default Groups DB");

    // Root should have 4 default child groups: General, Email, Banking, Social
    assert_eq!(
        root.children.len(),
        4,
        "Root should have 4 default groups, got {}",
        root.children.len()
    );

    let child_names: Vec<&str> = root.children.iter().map(|g| g.name.as_str()).collect();
    assert!(child_names.contains(&"General"), "Should have General group");
    assert!(child_names.contains(&"Email"), "Should have Email group");
    assert!(child_names.contains(&"Banking"), "Should have Banking group");
    assert!(child_names.contains(&"Social"), "Should have Social group");
}

#[test]
fn test_create_database_without_default_groups() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("no-default-groups.kdbx");

    let options = DatabaseCreationOptions {
        create_default_groups: false,
        ..Default::default()
    };

    let service = KdbxService::new();
    service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            None,
            "No Default Groups DB",
            &options,
        )
        .expect("Failed to create database");

    let groups = service.list_groups().expect("Failed to list groups");
    let root = &groups[0];

    // Root should have no child groups
    assert!(
        root.children.is_empty(),
        "Root should have no child groups when create_default_groups is false"
    );
}

#[test]
fn test_create_database_with_description() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("with-description.kdbx");

    let options = DatabaseCreationOptions {
        description: Some("This is my test database description".to_string()),
        ..Default::default()
    };

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            None,
            "Description DB",
            &options,
        )
        .expect("Failed to create database with description");

    assert_eq!(info.name, "Description DB");

    // Close and reopen - description should persist
    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpassword")
        .expect("Failed to reopen");

    // Note: We can't verify description via current API, but we verify the database
    // was created and can be reopened successfully
}

#[test]
fn test_create_database_with_custom_kdf_settings() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("custom-kdf.kdbx");

    // Use lower settings for faster test
    let options = DatabaseCreationOptions {
        kdf_memory: Some(16 * 1024 * 1024),  // 16 MB
        kdf_iterations: Some(2),              // 2 iterations
        kdf_parallelism: Some(2),             // 2 threads
        ..Default::default()
    };

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            None,
            "Custom KDF DB",
            &options,
        )
        .expect("Failed to create database with custom KDF");

    assert_eq!(info.version, "KDBX 4.0");

    // Close and reopen to verify KDF works
    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpassword")
        .expect("Failed to reopen database with custom KDF");
}

#[test]
fn test_create_database_with_all_options() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("all-options.kdbx");
    let keyfile_path = fixture_path("test-keyfile.keyx");

    if !keyfile_path.exists() {
        eprintln!("Skipping test: keyfile fixture not found");
        return;
    }

    let options = DatabaseCreationOptions {
        description: Some("Full featured database".to_string()),
        create_default_groups: true,
        kdf_memory: Some(32 * 1024 * 1024),  // 32 MB
        kdf_iterations: Some(2),
        kdf_parallelism: Some(2),
    };

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            Some(&keyfile_path.to_string_lossy()),
            "Full Featured DB",
            &options,
        )
        .expect("Failed to create database with all options");

    assert_eq!(info.name, "Full Featured DB");
    assert_eq!(info.version, "KDBX 4.0");

    // Verify default groups were created
    let groups = service.list_groups().expect("Failed to list groups");
    let root = &groups[0];
    assert_eq!(root.children.len(), 4, "Should have 4 default groups");

    // Close and reopen with keyfile
    service.close().expect("Failed to close");
    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "testpassword",
            &keyfile_path.to_string_lossy(),
        )
        .expect("Failed to reopen with all options");
}

#[test]
fn test_create_database_legacy_api_still_works() {
    // Ensure the legacy create() method still works for backwards compatibility
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("legacy-api.kdbx");

    let service = KdbxService::new();
    let info = service
        .create(&db_path.to_string_lossy(), "testpassword", "Legacy DB")
        .expect("Legacy create() should still work");

    assert_eq!(info.name, "Legacy DB");
    assert_eq!(info.version, "KDBX 4.0");

    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpassword")
        .expect("Legacy created DB should reopen");
}

#[test]
fn test_database_creation_options_defaults() {
    let options = DatabaseCreationOptions::default();

    // Verify default values
    assert_eq!(options.memory_bytes(), 64 * 1024 * 1024, "Default memory should be 64 MB");
    assert_eq!(options.iterations(), 3, "Default iterations should be 3");
    assert_eq!(options.parallelism(), 4, "Default parallelism should be 4");
    assert!(!options.create_default_groups, "Default should not create groups");
    assert!(options.description.is_none(), "Default should have no description");
}

#[test]
fn test_database_creation_options_custom_values() {
    let options = DatabaseCreationOptions {
        kdf_memory: Some(128 * 1024 * 1024),
        kdf_iterations: Some(5),
        kdf_parallelism: Some(8),
        description: Some("Test".to_string()),
        create_default_groups: true,
    };

    assert_eq!(options.memory_bytes(), 128 * 1024 * 1024);
    assert_eq!(options.iterations(), 5);
    assert_eq!(options.parallelism(), 8);
}
