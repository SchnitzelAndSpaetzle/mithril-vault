// SPDX-License-Identifier: MIT
//! Tests for entry command handlers
//!
//! These tests exercise the `KdbxService` methods that the entry commands delegate to.
//! The test structure mirrors the command API so that command-specific logic can be tested
//! when it is added.

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use mithril_vault_lib::models::database::DatabaseCreationOptions;
use mithril_vault_lib::models::entry::{CreateEntryData, UpdateEntryData};
use mithril_vault_lib::models::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;

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
    if !fixture_exists("test-kdbx4-low-KDF.kdbx") {
        return None;
    }

    let service = KdbxService::new();
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open test database");
    Some(service)
}

/// Helper to create a new database with default groups for CRUD tests
fn create_test_database() -> (KdbxService, TempDir) {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("entry-crud.kdbx");

    let options = DatabaseCreationOptions {
        create_default_groups: true,
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
            "Entry CRUD",
            &options,
        )
        .expect("Failed to create test database");

    (service, dir)
}

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

// ============================================================================
// create_entry command tests
// ============================================================================

#[test]
fn test_create_entry_success() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut custom_fields = BTreeMap::new();
    custom_fields.insert("Account".to_string(), "Personal".to_string());
    let mut protected_custom_fields = BTreeMap::new();
    protected_custom_fields.insert("PIN".to_string(), "1234".to_string());

    let data = CreateEntryData {
        title: "New Entry".to_string(),
        username: "user".to_string(),
        password: "secret".to_string(),
        url: Some("https://example.com".to_string()),
        notes: Some("Notes".to_string()),
        icon_id: Some(1),
        tags: Some(vec!["tag1".to_string()]),
        custom_fields: Some(custom_fields),
        protected_custom_fields: Some(protected_custom_fields),
    };

    let entry = service
        .create_entry(&info.root_group_id, data)
        .expect("create entry");

    assert_eq!(entry.title, "New Entry");
    assert_eq!(entry.group_id, info.root_group_id);
    assert_eq!(entry.icon_id, Some(1));
    assert_eq!(entry.tags, vec!["tag1".to_string()]);
    assert_eq!(
        entry.custom_fields.get("Account").map(String::as_str),
        Some("Personal")
    );
    assert!(
        !entry.custom_fields.contains_key("PIN"),
        "Protected custom fields should not be returned in custom_fields"
    );
    assert!(
        entry
            .custom_field_meta
            .iter()
            .any(|meta| meta.key == "PIN" && meta.is_protected),
        "Protected custom fields should be surfaced as metadata"
    );

    let password = service
        .get_entry_password(&entry.id)
        .expect("entry password");
    assert_eq!(password, "secret");

    let protected = service
        .get_entry_protected_custom_field(&entry.id, "PIN")
        .expect("protected custom field");
    assert_eq!(protected.value, "1234");
}

#[test]
fn test_create_entry_group_not_found() {
    let (service, _dir) = create_test_database();

    let data = CreateEntryData {
        title: "New Entry".to_string(),
        username: "user".to_string(),
        password: "secret".to_string(),
        url: None,
        notes: None,
        icon_id: None,
        tags: None,
        custom_fields: None,
        protected_custom_fields: None,
    };

    let result = service.create_entry("missing-group", data);

    assert!(
        matches!(result, Err(AppError::GroupNotFound(_))),
        "Should fail with GroupNotFound for invalid group ID"
    );
}

#[test]
fn test_get_entry_protected_custom_field_requires_protection() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut custom_fields = BTreeMap::new();
    custom_fields.insert("Hint".to_string(), "Visible".to_string());

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Entry".to_string(),
                username: "user".to_string(),
                password: "secret".to_string(),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: Some(custom_fields),
                protected_custom_fields: None,
            },
        )
        .expect("create entry");

    let result = service.get_entry_protected_custom_field(&entry.id, "Hint");

    assert!(
        matches!(result, Err(AppError::CustomFieldNotProtected(_))),
        "Should error when requesting an unprotected custom field"
    );
}

// ============================================================================
// update_entry command tests
// ============================================================================

#[test]
fn test_update_entry_success() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let data = CreateEntryData {
        title: "Original".to_string(),
        username: "user".to_string(),
        password: "secret".to_string(),
        url: None,
        notes: None,
        icon_id: None,
        tags: None,
        custom_fields: None,
        protected_custom_fields: None,
    };

    let entry = service
        .create_entry(&info.root_group_id, data)
        .expect("create entry");
    let original_modified = entry.modified_at.clone();

    sleep(Duration::from_secs(1));

    let mut custom_fields = BTreeMap::new();
    custom_fields.insert("Category".to_string(), "Work".to_string());
    let mut protected_custom_fields = BTreeMap::new();
    protected_custom_fields.insert("PIN".to_string(), "5678".to_string());

    let updated = service
        .update_entry(
            &entry.id,
            UpdateEntryData {
                title: Some("Updated".to_string()),
                username: None,
                password: Some("new-secret".to_string()),
                url: Some("https://updated.example.com".to_string()),
                notes: Some("Updated notes".to_string()),
                icon_id: Some(2),
                tags: Some(vec!["tag2".to_string()]),
                custom_fields: Some(custom_fields),
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("update entry");

    assert_eq!(updated.title, "Updated");
    assert_eq!(updated.icon_id, Some(2));
    assert_eq!(updated.tags, vec!["tag2".to_string()]);
    assert_eq!(
        updated.custom_fields.get("Category").map(String::as_str),
        Some("Work")
    );
    assert!(
        !updated.custom_fields.contains_key("PIN"),
        "Protected custom fields should not be returned in custom_fields"
    );
    assert_ne!(
        updated.modified_at, original_modified,
        "modified_at should update on changes"
    );

    let password = service
        .get_entry_password(&entry.id)
        .expect("updated password");
    assert_eq!(password, "new-secret");

    let protected = service
        .get_entry_protected_custom_field(&entry.id, "PIN")
        .expect("protected custom field");
    assert_eq!(protected.value, "5678");
}

#[test]
fn test_update_entry_not_found() {
    let (service, _dir) = create_test_database();

    let result = service.update_entry(
        "missing-entry",
        UpdateEntryData {
            title: Some("Updated".to_string()),
            username: None,
            password: None,
            url: None,
            notes: None,
            icon_id: None,
            tags: None,
            custom_fields: None,
            protected_custom_fields: None,
        },
    );

    assert!(
        matches!(result, Err(AppError::EntryNotFound(_))),
        "Should fail with EntryNotFound for invalid entry ID"
    );
}

// ============================================================================
// delete_entry command tests
// ============================================================================

#[test]
fn test_delete_entry_moves_to_recycle_bin() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Disposable".to_string(),
                username: "user".to_string(),
                password: "secret".to_string(),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
            },
        )
        .expect("create entry");

    let root_entries = service
        .list_entries(Some(&info.root_group_id))
        .expect("root entries");
    assert!(
        root_entries.iter().any(|item| item.id == entry.id),
        "Entry should exist in root before delete"
    );

    service.delete_entry(&entry.id).expect("delete entry");

    let root_entries_after = service
        .list_entries(Some(&info.root_group_id))
        .expect("root entries after");
    assert!(
        !root_entries_after.iter().any(|item| item.id == entry.id),
        "Entry should be removed from root after delete"
    );

    let all_entries = service.list_entries(None).expect("all entries");
    assert!(
        all_entries.iter().any(|item| item.id == entry.id),
        "Entry should remain in database after delete"
    );

    let moved_entry = service.get_entry(&entry.id).expect("moved entry");
    assert_ne!(
        moved_entry.group_id, info.root_group_id,
        "Deleted entry should move to a different group"
    );
}

#[test]
fn test_delete_entry_not_found() {
    let (service, _dir) = create_test_database();

    let result = service.delete_entry("missing-entry");

    assert!(
        matches!(result, Err(AppError::EntryNotFound(_))),
        "Should fail with EntryNotFound for invalid entry ID"
    );
}

// ============================================================================
// move_entry command tests
// ============================================================================

#[test]
fn test_move_entry_success() {
    let (service, _dir) = create_test_database();
    let groups = service.list_groups().expect("groups");
    let root_group = &groups[0];
    let target_group = root_group
        .children
        .first()
        .expect("default group available");

    let entry = service
        .create_entry(
            &root_group.id,
            CreateEntryData {
                title: "Movable".to_string(),
                username: "user".to_string(),
                password: "secret".to_string(),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
            },
        )
        .expect("create entry");

    let moved = service
        .move_entry(&entry.id, &target_group.id)
        .expect("move entry");
    assert_eq!(moved.group_id, target_group.id);

    let root_entries = service
        .list_entries(Some(&root_group.id))
        .expect("root entries");
    assert!(
        !root_entries.iter().any(|item| item.id == entry.id),
        "Entry should not remain in root after move"
    );

    let target_entries = service
        .list_entries(Some(&target_group.id))
        .expect("target entries");
    assert!(
        target_entries.iter().any(|item| item.id == entry.id),
        "Entry should exist in target group after move"
    );
}
