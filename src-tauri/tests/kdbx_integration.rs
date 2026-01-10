// SPDX-License-Identifier: GPL-3.0-or-later
//! Integration tests for KDBX database operations via KdbxService

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use mithril_vault_lib::models::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use std::path::PathBuf;

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
    let entry = service
        .get_entry(&entry_id)
        .expect("Failed to fetch entry");
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
fn test_create_and_save_not_implemented() {
    let service = KdbxService::new();
    let create_result = service.create("path", "password", "name");
    assert!(
        matches!(create_result, Err(AppError::NotImplemented(_))),
        "Create should be marked not implemented"
    );

    let save_result = service.save();
    assert!(
        matches!(save_result, Err(AppError::NotImplemented(_))),
        "Save should be marked not implemented"
    );
}
