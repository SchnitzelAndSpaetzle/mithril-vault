// SPDX-License-Identifier: MIT
//! Tests for group command handlers
//!
//! These tests exercise the `KdbxService` methods that the group commands delegate to.
//! The test structure mirrors the command API so that command-specific logic can be tested
//! when it is added.

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

/// Helper to check if test fixtures exist
fn fixture_exists(filename: &str) -> bool {
    fixture_path(filename).exists()
}

/// Helper to create a service with an open test database
fn open_test_database() -> Option<KdbxService> {
    if !fixture_exists("test-kdbx4.kdbx") {
        return None;
    }

    let service = KdbxService::new();
    let path = fixture_path("test-kdbx4.kdbx");
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open test database");
    Some(service)
}

// ============================================================================
// list_groups command tests
// ============================================================================

#[test]
fn test_list_groups_success() {
    let Some(service) = open_test_database() else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let result = service.list_groups();

    assert!(result.is_ok(), "Should successfully list groups");
    let groups = result.expect("groups");
    assert!(!groups.is_empty(), "Should have at least the root group");

    // Verify root group structure
    let root = &groups[0];
    assert!(!root.id.is_empty(), "Root group should have an ID");
    assert!(
        root.parent_id.is_none(),
        "Root group should not have a parent"
    );
}

#[test]
fn test_list_groups_database_not_open() {
    let service = KdbxService::new();

    let result = service.list_groups();

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open"
    );
}

// ============================================================================
// get_group command tests
// ============================================================================

#[test]
fn test_get_group_success() {
    let Some(service) = open_test_database() else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    // Get the root group ID
    let info = service.get_info().expect("database info");
    let root_group_id = &info.root_group_id;

    let result = service.get_group(root_group_id);

    assert!(result.is_ok(), "Should successfully get group by ID");
    let group = result.expect("group");
    assert_eq!(
        group.id, *root_group_id,
        "Group ID should match requested ID"
    );
}

#[test]
fn test_get_group_not_found() {
    let Some(service) = open_test_database() else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let result = service.get_group("nonexistent-group-id");

    assert!(
        matches!(result, Err(AppError::GroupNotFound(_))),
        "Should fail with GroupNotFound for invalid ID"
    );
}

#[test]
fn test_get_group_database_not_open() {
    let service = KdbxService::new();

    let result = service.get_group("some-id");

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open"
    );
}
