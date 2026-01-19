// SPDX-License-Identifier: MIT
//! Tests for entry command handlers
//!
//! These tests exercise the `KdbxService` methods that the entry commands delegate to.
//! The test structure mirrors the command API so that command-specific logic can be tested
//! when it is added.

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;

use super::open_test_database;

// ============================================================================
// list_entries command tests
// ============================================================================

#[test]
fn test_list_entries_all() {
    let Some(service) = open_test_database() else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let result = service.list_entries(None);

    assert!(result.is_ok(), "Should successfully list all entries");
    let entries = result.expect("entries");
    assert!(!entries.is_empty(), "Test fixture should have entries");

    // Verify entry structure
    let first_entry = &entries[0];
    assert!(!first_entry.id.is_empty(), "Entry should have an ID");
    assert!(
        !first_entry.group_id.is_empty(),
        "Entry should have a group ID"
    );
}

#[test]
fn test_list_entries_by_group() {
    let Some(service) = open_test_database() else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    // Get the root group ID
    let info = service.get_info().expect("database info");
    let root_group_id = info.root_group_id;

    let result = service.list_entries(Some(&root_group_id));

    assert!(result.is_ok(), "Should successfully list entries by group");
    let entries = result.expect("entries");

    // Entries in root may be fewer than all entries (if subgroups exist)
    let all_entries = service.list_entries(None).expect("all entries");
    assert!(
        entries.len() <= all_entries.len(),
        "Group-filtered entries should not exceed total entries"
    );
}

#[test]
fn test_list_entries_database_not_open() {
    let service = KdbxService::new();

    let result = service.list_entries(None);

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open"
    );
}

// ============================================================================
// get_entry command tests
// ============================================================================

#[test]
fn test_get_entry_success() {
    let Some(service) = open_test_database() else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    // First get an entry ID from the list
    let entries = service.list_entries(None).expect("entries");
    assert!(!entries.is_empty(), "Need at least one entry for test");
    let entry_id = &entries[0].id;

    let result = service.get_entry(entry_id);

    assert!(result.is_ok(), "Should successfully get entry by ID");
    let entry = result.expect("entry");
    assert_eq!(entry.id, *entry_id, "Entry ID should match requested ID");
    assert_eq!(
        entry.group_id, entries[0].group_id,
        "Group ID should match list item"
    );
}

#[test]
fn test_get_entry_not_found() {
    let Some(service) = open_test_database() else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let result = service.get_entry("nonexistent-entry-id");

    assert!(
        matches!(result, Err(AppError::EntryNotFound(_))),
        "Should fail with EntryNotFound for invalid ID"
    );
}

#[test]
fn test_get_entry_database_not_open() {
    let service = KdbxService::new();

    let result = service.get_entry("some-id");

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open"
    );
}

// ============================================================================
// get_entry_password command tests
// ============================================================================

#[test]
fn test_get_entry_password_success() {
    let Some(service) = open_test_database() else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    // First get an entry ID from the list
    let entries = service.list_entries(None).expect("entries");
    assert!(!entries.is_empty(), "Need at least one entry for test");
    let entry_id = &entries[0].id;

    let result = service.get_entry_password(entry_id);

    assert!(result.is_ok(), "Should successfully get entry password");
    let password = result.expect("password");
    // Test fixture entries should have passwords set
    assert!(
        !password.is_empty(),
        "Test fixture entry should have a password"
    );
}

#[test]
fn test_get_entry_password_not_found() {
    let Some(service) = open_test_database() else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let result = service.get_entry_password("nonexistent-entry-id");

    assert!(
        matches!(result, Err(AppError::EntryNotFound(_))),
        "Should fail with EntryNotFound for invalid ID"
    );
}

#[test]
fn test_get_entry_password_database_not_open() {
    let service = KdbxService::new();

    let result = service.get_entry_password("some-id");

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open"
    );
}
