// SPDX-License-Identifier: MIT
//! Tests for database command handlers
//!
//! These tests exercise the `KdbxService` methods that the database commands delegate to.
//! The test structure mirrors the command API so that command-specific logic can be tested
//! when it is added.

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::file_lock::FileLockService;
use mithril_vault_lib::services::kdbx::KdbxService;
use tempfile::tempdir;

use super::copy_fixture_to_temp;

// ============================================================================
// open_database command tests
// ============================================================================

#[test]
fn test_open_database_success() {
    let Some((_temp_dir, db_path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let service = KdbxService::new();

    let result = service.open(&db_path.to_string_lossy(), "test123");

    assert!(result.is_ok(), "Should successfully open database");
    let info = result.expect("database info");
    assert!(!info.name.is_empty(), "Database should have a name");
    assert!(
        !info.root_group_id.is_empty(),
        "Database should have a root group"
    );
    assert!(
        !info.is_modified,
        "Newly opened database should not be modified"
    );
}

#[test]
fn test_open_database_invalid_password() {
    let Some((_temp_dir, db_path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let service = KdbxService::new();

    let result = service.open(&db_path.to_string_lossy(), "wrong_password");

    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Should fail with InvalidPassword error for wrong password"
    );
}

#[test]
fn test_open_database_file_not_found() {
    let service = KdbxService::new();
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("nonexistent-database.kdbx");

    let result = service.open(&path.to_string_lossy(), "test123");

    assert!(
        matches!(result, Err(AppError::InvalidPath(_))),
        "Should fail with InvalidPath error for missing file"
    );
}

// ============================================================================
// close_database command tests
// ============================================================================

#[test]
fn test_close_database_success() {
    let Some((_temp_dir, db_path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let service = KdbxService::new();

    service
        .open(&db_path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    let result = service.close();

    assert!(result.is_ok(), "Should successfully close database");

    // Verify database is closed by checking get_info fails
    let info_result = service.get_info();
    assert!(
        matches!(info_result, Err(AppError::DatabaseNotOpen)),
        "Database should be closed"
    );
}

#[test]
fn test_close_database_not_open() {
    let service = KdbxService::new();

    let result = service.close();

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen error when no database is open"
    );
}

// ============================================================================
// create_database command tests
// ============================================================================

#[test]
fn test_create_database_success() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("new-test-database.kdbx");

    let service = KdbxService::new();

    let result = service.create(&db_path.to_string_lossy(), "newpassword", "My Test Vault");

    assert!(result.is_ok(), "Should successfully create database");
    let info = result.expect("database info");
    assert_eq!(info.name, "My Test Vault");
    assert!(!info.root_group_id.is_empty());
    assert!(
        !info.is_modified,
        "Newly created database should not be modified"
    );
    assert!(db_path.exists(), "Database file should exist on disk");
}

#[test]
fn test_create_database_already_open() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path1 = dir.path().join("first.kdbx");
    let db_path2 = dir.path().join("second.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path1.to_string_lossy(), "pass1", "First DB")
        .expect("Failed to create first database");

    let result = service.create(&db_path2.to_string_lossy(), "pass2", "Second DB");

    assert!(
        matches!(result, Err(AppError::DatabaseAlreadyOpen)),
        "Should fail with DatabaseAlreadyOpen when trying to create while one is open"
    );
}

// ============================================================================
// save_database command tests
// ============================================================================

#[test]
fn test_save_database_success() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("save-test.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path.to_string_lossy(), "savepass", "Save Test DB")
        .expect("Failed to create database");

    let result = service.save();

    assert!(result.is_ok(), "Should successfully save database");

    // Verify file can be reopened
    service.close().expect("Failed to close");
    let reopen_result = service.open(&db_path.to_string_lossy(), "savepass");
    assert!(
        reopen_result.is_ok(),
        "Should be able to reopen saved database"
    );
}

#[test]
fn test_save_database_not_open() {
    let service = KdbxService::new();

    let result = service.save();

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open"
    );
}

#[test]
fn test_save_as_moves_lock_file() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("original.kdbx");
    let new_path = dir.path().join("moved.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path.to_string_lossy(), "savepass", "Save Test DB")
        .expect("Failed to create database");

    let db_path_str = db_path.to_string_lossy();
    let old_lock_path = FileLockService::lock_file_path(db_path_str.as_ref());
    assert!(old_lock_path.exists(), "Original lock file should exist");

    service
        .save_as(&new_path.to_string_lossy(), None)
        .expect("Failed to save database as new path");

    let new_path_str = new_path.to_string_lossy();
    let new_lock_path = FileLockService::lock_file_path(new_path_str.as_ref());
    assert!(
        !old_lock_path.exists(),
        "Old lock file should be removed after save_as"
    );
    assert!(
        new_lock_path.exists(),
        "New lock file should exist after save_as"
    );

    service.close().expect("Failed to close database");
    assert!(
        !new_lock_path.exists(),
        "Lock file should be removed after close"
    );
}
