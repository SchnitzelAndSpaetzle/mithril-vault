// SPDX-License-Identifier: GPL-3.0-or-later
//! Integration tests for KDBX database operations via KdbxService

#![allow(clippy::expect_used)] // expect() is acceptable in tests

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
    let path = fixture_path("test-kdbx4.kdbx");
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
    let path = fixture_path("test-kdbx3.kdbx");
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
fn test_open_with_keyfile() {
    let db_path = fixture_path("test-keyfile-kdbx4.kdbx");
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
    let path = fixture_path("test-kdbx4.kdbx");
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
    let path = fixture_path("test-kdbx4.kdbx");
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

    let password = service
        .get_entry_password(&entry_id)
        .expect("Failed to fetch entry password");
    let _ = password;

    let entries_in_root = service
        .list_entries(Some(&info.root_group_id))
        .expect("Failed to list entries by group");
    assert!(entries_in_root.len() <= entries.len());
}

#[test]
fn test_list_groups_and_get_group() {
    let path = fixture_path("test-kdbx4.kdbx");
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
    let path = fixture_path("test-kdbx4.kdbx");
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
    let path = fixture_path("test-kdbx4.kdbx");
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
    let path = fixture_path("test-kdbx4.kdbx");
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
