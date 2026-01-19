// SPDX-License-Identifier: MIT
//! Tests for database command handlers
//!
//! These tests exercise the `KdbxService` methods that the database commands delegate to.
//! The test structure mirrors the command API so that command-specific logic can be tested
//! when it is added.

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use tempfile::tempdir;

use super::{fixture_exists, fixture_path};

// ============================================================================
// open_database command tests
// ============================================================================

#[test]
fn test_open_database_success() {
    if !fixture_exists("test-kdbx4-low-KDF.kdbx") {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let service = KdbxService::new();
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");

    let result = service.open(&path.to_string_lossy(), "test123");

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
    if !fixture_exists("test-kdbx4-low-KDF.kdbx") {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let service = KdbxService::new();
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");

    let result = service.open(&path.to_string_lossy(), "wrong_password");

    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Should fail with InvalidPassword error for wrong password"
    );
}

#[test]
fn test_open_database_file_not_found() {
    let service = KdbxService::new();
    let path = fixture_path("nonexistent-database.kdbx");

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
    if !fixture_exists("test-kdbx4-low-KDF.kdbx") {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let service = KdbxService::new();
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");

    service
        .open(&path.to_string_lossy(), "test123")
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
