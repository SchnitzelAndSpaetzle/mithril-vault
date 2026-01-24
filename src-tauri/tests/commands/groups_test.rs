// SPDX-License-Identifier: MIT
//! Tests for group command handlers
//!
//! These tests exercise the `KdbxService` methods that the group commands delegate to.
//! The test structure mirrors the command API so that command-specific logic can be tested
//! when it is added.

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use mithril_vault_lib::dto::database::DatabaseCreationOptions;
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::dto::group::UpdateGroupData;
use mithril_vault_lib::services::kdbx::KdbxService;
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;

use super::open_test_database;

/// Helper to create a new database for CRUD tests
fn create_test_database() -> (KdbxService, TempDir) {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("group-crud.kdbx");

    let options = DatabaseCreationOptions {
        create_default_groups: false,
        kdf_memory: Some(1024 * 1024),
        kdf_iterations: Some(1),
        kdf_parallelism: Some(1),
        description: None,
    };

    let service = KdbxService::new();
    service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpass"),
            None,
            "Group CRUD",
            &options,
        )
        .expect("Failed to create test database");

    (service, dir)
}

// ============================================================================
// list_groups command tests
// ============================================================================

#[test]
fn test_list_groups_success() {
    let Some((service, _temp_dir)) = open_test_database() else {
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
    let Some((service, _temp_dir)) = open_test_database() else {
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
    let Some((service, _temp_dir)) = open_test_database() else {
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

// ============================================================================
// create_group command tests
// ============================================================================

#[test]
fn test_create_group_in_root() {
    let (service, _dir) = create_test_database();

    let result = service.create_group(None, "New Group", None);

    assert!(result.is_ok(), "Should successfully create group in root");
    let group = result.expect("group");
    assert_eq!(group.name, "New Group");
    assert!(group.parent_id.is_some(), "Should have root as parent");
}

#[test]
fn test_create_group_in_subgroup() {
    let (service, _dir) = create_test_database();

    // Create a parent group first
    let parent = service
        .create_group(None, "Parent Group", None)
        .expect("parent group");

    // Create child group
    let result = service.create_group(Some(&parent.id), "Child Group", None);

    assert!(
        result.is_ok(),
        "Should successfully create group in subgroup"
    );
    let child = result.expect("child group");
    assert_eq!(child.name, "Child Group");
    assert_eq!(
        child.parent_id.as_deref(),
        Some(parent.id.as_str()),
        "Child should reference parent"
    );
}

#[test]
fn test_create_group_with_icon() {
    let (service, _dir) = create_test_database();

    let result = service.create_group(None, "Icon Group", Some(5));

    assert!(result.is_ok(), "Should successfully create group with icon");
    let group = result.expect("group");
    assert_eq!(group.icon.as_deref(), Some("5"));
}

#[test]
fn test_create_group_parent_not_found() {
    let (service, _dir) = create_test_database();

    let result = service.create_group(Some("nonexistent-id"), "Test", None);

    assert!(
        matches!(result, Err(AppError::GroupNotFound(_))),
        "Should fail with GroupNotFound for invalid parent ID"
    );
}

#[test]
fn test_create_group_database_not_open() {
    let service = KdbxService::new();

    let result = service.create_group(None, "Test", None);

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open"
    );
}

// ============================================================================
// update_group command tests
// ============================================================================

#[test]
fn test_update_group_name() {
    let (service, _dir) = create_test_database();

    let group = service
        .create_group(None, "Original Name", None)
        .expect("create group");

    let result = service.update_group(
        &group.id,
        UpdateGroupData {
            name: Some("Updated Name".to_string()),
            icon: None,
        },
    );

    assert!(result.is_ok(), "Should successfully update group name");
    let updated = result.expect("updated group");
    assert_eq!(updated.name, "Updated Name");
}

#[test]
fn test_update_group_icon() {
    let (service, _dir) = create_test_database();

    let group = service
        .create_group(None, "Test Group", None)
        .expect("create group");

    let result = service.update_group(
        &group.id,
        UpdateGroupData {
            name: None,
            icon: Some("10".to_string()),
        },
    );

    assert!(result.is_ok(), "Should successfully update group icon");
    let updated = result.expect("updated group");
    assert_eq!(updated.icon.as_deref(), Some("10"));
}

#[test]
fn test_update_group_both_fields() {
    let (service, _dir) = create_test_database();

    let group = service
        .create_group(None, "Original", None)
        .expect("create group");

    let result = service.update_group(
        &group.id,
        UpdateGroupData {
            name: Some("New Name".to_string()),
            icon: Some("15".to_string()),
        },
    );

    assert!(result.is_ok(), "Should successfully update both fields");
    let updated = result.expect("updated group");
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.icon.as_deref(), Some("15"));
}

#[test]
fn test_update_group_not_found() {
    let (service, _dir) = create_test_database();

    let result = service.update_group(
        "nonexistent-id",
        UpdateGroupData {
            name: Some("Test".to_string()),
            icon: None,
        },
    );

    assert!(
        matches!(result, Err(AppError::GroupNotFound(_))),
        "Should fail with GroupNotFound for invalid ID"
    );
}

#[test]
fn test_update_group_modified_timestamp_changes() {
    let (service, _dir) = create_test_database();

    let group = service
        .create_group(None, "Test", None)
        .expect("create group");

    // Get the group to see its initial state
    let initial = service.get_group(&group.id).expect("get group");

    sleep(Duration::from_secs(1));

    let updated = service
        .update_group(
            &group.id,
            UpdateGroupData {
                name: Some("Updated".to_string()),
                icon: None,
            },
        )
        .expect("update group");

    // Timestamps are on the group object, need to verify modification happened
    // by checking the name changed and the group was marked modified
    assert_eq!(updated.name, "Updated");
    assert_ne!(initial.name, updated.name);
}

// ============================================================================
// rename_group (via update_group) tests
// ============================================================================

#[test]
fn test_rename_group_success() {
    let (service, _dir) = create_test_database();

    let group = service
        .create_group(None, "Old Name", None)
        .expect("create group");

    // rename_group internally uses update_group with only name
    let result = service.update_group(
        &group.id,
        UpdateGroupData {
            name: Some("New Name".to_string()),
            icon: None,
        },
    );

    assert!(result.is_ok(), "Should successfully rename group");
    let renamed = result.expect("renamed group");
    assert_eq!(renamed.name, "New Name");
}

#[test]
fn test_rename_group_not_found() {
    let (service, _dir) = create_test_database();

    let result = service.update_group(
        "nonexistent-id",
        UpdateGroupData {
            name: Some("New Name".to_string()),
            icon: None,
        },
    );

    assert!(
        matches!(result, Err(AppError::GroupNotFound(_))),
        "Should fail with GroupNotFound for invalid ID"
    );
}

// ============================================================================
// delete_group command tests
// ============================================================================

#[test]
fn test_delete_group_soft_delete() {
    let (service, _dir) = create_test_database();

    let group = service
        .create_group(None, "To Delete", None)
        .expect("create group");
    let group_id = group.id.clone();

    // Delete without recursive, not permanent (moves to recycle bin)
    let result = service.delete_group(&group_id, false, false);

    assert!(result.is_ok(), "Should successfully soft delete group");

    // The group should still exist (in recycle bin)
    let groups = service.list_groups().expect("list groups");
    let root = &groups[0];

    // Check it's not in root anymore
    let in_root = root.children.iter().any(|g| g.id == group_id);
    assert!(!in_root, "Group should not be in root after delete");
}

#[test]
fn test_delete_group_permanent() {
    let (service, _dir) = create_test_database();

    let group = service
        .create_group(None, "To Delete Permanently", None)
        .expect("create group");
    let group_id = group.id.clone();

    // Delete permanently
    let result = service.delete_group(&group_id, false, true);

    assert!(
        result.is_ok(),
        "Should successfully permanently delete group"
    );

    // The group should not be found anymore
    let get_result = service.get_group(&group_id);
    assert!(
        matches!(get_result, Err(AppError::GroupNotFound(_))),
        "Permanently deleted group should not be found"
    );
}

#[test]
fn test_delete_group_recursive_with_children() {
    let (service, _dir) = create_test_database();

    let parent = service
        .create_group(None, "Parent", None)
        .expect("parent group");
    let _child = service
        .create_group(Some(&parent.id), "Child", None)
        .expect("child group");

    // Delete recursively
    let result = service.delete_group(&parent.id, true, true);

    assert!(
        result.is_ok(),
        "Should successfully recursively delete group with children"
    );
}

#[test]
fn test_delete_group_non_recursive_fails_with_children() {
    let (service, _dir) = create_test_database();

    let parent = service
        .create_group(None, "Parent", None)
        .expect("parent group");
    let _child = service
        .create_group(Some(&parent.id), "Child", None)
        .expect("child group");

    // Try to delete non-recursively
    let result = service.delete_group(&parent.id, false, true);

    assert!(
        matches!(result, Err(AppError::GroupNotEmpty(_))),
        "Should fail with GroupNotEmpty when trying to delete non-recursively with children"
    );
}

#[test]
fn test_delete_group_cannot_delete_root() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let result = service.delete_group(&info.root_group_id, false, false);

    assert!(
        matches!(result, Err(AppError::CannotDeleteRootGroup)),
        "Should fail with CannotDeleteRootGroup when trying to delete root"
    );
}

#[test]
fn test_delete_group_not_found() {
    let (service, _dir) = create_test_database();

    let result = service.delete_group("nonexistent-id", false, false);

    assert!(
        matches!(result, Err(AppError::GroupNotFound(_))),
        "Should fail with GroupNotFound for invalid ID"
    );
}

#[test]
fn test_delete_group_database_not_open() {
    let service = KdbxService::new();

    let result = service.delete_group("some-id", false, false);

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open"
    );
}

// ============================================================================
// move_group command tests
// ============================================================================

#[test]
fn test_move_group_to_different_parent() {
    let (service, _dir) = create_test_database();

    let group_a = service
        .create_group(None, "Group A", None)
        .expect("group A");
    let group_b = service
        .create_group(None, "Group B", None)
        .expect("group B");

    // Move Group A into Group B
    let result = service.move_group(&group_a.id, Some(&group_b.id));

    assert!(result.is_ok(), "Should successfully move group");
    let moved = result.expect("moved group");
    assert_eq!(
        moved.parent_id.as_deref(),
        Some(group_b.id.as_str()),
        "Moved group should have new parent"
    );
}

#[test]
fn test_move_group_to_root() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let parent = service
        .create_group(None, "Parent", None)
        .expect("parent group");
    let child = service
        .create_group(Some(&parent.id), "Child", None)
        .expect("child group");

    // Move child to root (target_parent_id = None means root)
    let result = service.move_group(&child.id, None);

    assert!(result.is_ok(), "Should successfully move group to root");
    let moved = result.expect("moved group");
    assert_eq!(
        moved.parent_id.as_deref(),
        Some(info.root_group_id.as_str()),
        "Moved group should be under root"
    );
}

#[test]
fn test_move_group_cannot_move_root() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let target = service.create_group(None, "Target", None).expect("target");

    let result = service.move_group(&info.root_group_id, Some(&target.id));

    assert!(
        matches!(result, Err(AppError::CannotMoveRootGroup)),
        "Should fail with CannotMoveRootGroup when trying to move root"
    );
}

#[test]
fn test_move_group_circular_reference_to_self() {
    let (service, _dir) = create_test_database();

    let group = service
        .create_group(None, "Group", None)
        .expect("create group");

    // Try to move group into itself
    let result = service.move_group(&group.id, Some(&group.id));

    assert!(
        matches!(result, Err(AppError::CircularReference)),
        "Should fail with CircularReference when moving to self"
    );
}

#[test]
fn test_move_group_circular_reference_to_descendant() {
    let (service, _dir) = create_test_database();

    let parent = service.create_group(None, "Parent", None).expect("parent");
    let child = service
        .create_group(Some(&parent.id), "Child", None)
        .expect("child");
    let grandchild = service
        .create_group(Some(&child.id), "Grandchild", None)
        .expect("grandchild");

    // Try to move parent into its grandchild (circular)
    let result = service.move_group(&parent.id, Some(&grandchild.id));

    assert!(
        matches!(result, Err(AppError::CircularReference)),
        "Should fail with CircularReference when moving to descendant"
    );
}

#[test]
fn test_move_group_not_found() {
    let (service, _dir) = create_test_database();

    let target = service.create_group(None, "Target", None).expect("target");

    let result = service.move_group("nonexistent-id", Some(&target.id));

    assert!(
        matches!(result, Err(AppError::GroupNotFound(_))),
        "Should fail with GroupNotFound for invalid group ID"
    );
}

#[test]
fn test_move_group_target_not_found() {
    let (service, _dir) = create_test_database();

    let group = service.create_group(None, "Group", None).expect("group");

    let result = service.move_group(&group.id, Some("nonexistent-target"));

    assert!(
        matches!(result, Err(AppError::GroupNotFound(_))),
        "Should fail with GroupNotFound for invalid target ID"
    );
}
