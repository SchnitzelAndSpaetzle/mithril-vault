// SPDX-License-Identifier: MIT
//! Tests for entry command handlers
//!
//! These tests exercise the `KdbxService` methods that the entry commands delegate to.
//! The test structure mirrors the command API so that command-specific logic can be tested
//! when it is added.

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use mithril_vault_lib::domain::secure::SecureString;
use mithril_vault_lib::dto::database::DatabaseCreationOptions;
use mithril_vault_lib::dto::entry::{CreateEntryData, UpdateEntryData};
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use std::collections::BTreeMap;
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;

use super::open_test_database;

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
    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert("PIN".to_string(), SecureString::from("1234"));

    let data = CreateEntryData {
        title: "New Entry".to_string(),
        username: "user".to_string(),
        password: SecureString::from("secret"),
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
        password: SecureString::from("secret"),
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
                password: SecureString::from("secret"),
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
        password: SecureString::from("secret"),
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
    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert("PIN".to_string(), SecureString::from("5678"));

    let updated = service
        .update_entry(
            &entry.id,
            UpdateEntryData {
                title: Some("Updated".to_string()),
                username: None,
                password: Some(SecureString::from("new-secret")),
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
                password: SecureString::from("secret"),
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
                password: SecureString::from("secret"),
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

// ============================================================================
// Protected custom field tests
// ============================================================================

#[test]
fn test_protected_custom_field_roundtrip_save_reopen() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("roundtrip.kdbx");

    let options = DatabaseCreationOptions {
        create_default_groups: false,
        kdf_memory: Some(1024 * 1024),
        kdf_iterations: Some(1),
        kdf_parallelism: Some(1),
        description: None,
    };

    // Create database and add entry with protected field
    let service = KdbxService::new();
    service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpass"),
            None,
            "Roundtrip Test",
            &options,
        )
        .expect("Failed to create test database");

    let info = service.get_info().expect("database info");

    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert(
        "SecretKey".to_string(),
        SecureString::from("my-secret-value-123"),
    );

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Roundtrip Entry".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("create entry");

    let entry_id = entry.id.clone();

    // Save database
    service.save().expect("save database");

    // Close database
    let _ = service.close();

    // Reopen database
    service
        .open(&db_path.to_string_lossy(), "testpass")
        .expect("reopen database");

    // Verify protected field persisted
    let protected = service
        .get_entry_protected_custom_field(&entry_id, "SecretKey")
        .expect("get protected field after reopen");

    assert_eq!(
        protected.value, "my-secret-value-123",
        "Protected field value should persist after save/reopen"
    );
}

#[test]
fn test_protected_custom_field_empty_value() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert("EmptySecret".to_string(), SecureString::from(""));

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Empty Protected Field".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("create entry");

    let protected = service
        .get_entry_protected_custom_field(&entry.id, "EmptySecret")
        .expect("get empty protected field");

    assert_eq!(
        protected.value, "",
        "Empty protected field should be retrievable"
    );
}

#[test]
fn test_protected_custom_field_unicode_value() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    // Test with emojis, CJK characters, and other Unicode
    protected_custom_fields.insert(
        "UnicodeSecret".to_string(),
        SecureString::from("密码🔐パスワード🗝️Contraseña"),
    );

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Unicode Protected Field".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("create entry");

    let protected = service
        .get_entry_protected_custom_field(&entry.id, "UnicodeSecret")
        .expect("get unicode protected field");

    assert_eq!(
        protected.value, "密码🔐パスワード🗝️Contraseña",
        "Unicode protected field should be retrievable with correct characters"
    );
}

#[test]
fn test_protected_custom_field_special_characters() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    // Test with XML special chars and other special characters
    protected_custom_fields.insert(
        "SpecialSecret".to_string(),
        SecureString::from("<>&\"'{}[]|\\`~!@#$%^&*()"),
    );

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Special Chars Protected Field".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("create entry");

    let protected = service
        .get_entry_protected_custom_field(&entry.id, "SpecialSecret")
        .expect("get special chars protected field");

    assert_eq!(
        protected.value, "<>&\"'{}[]|\\`~!@#$%^&*()",
        "Protected field with special characters should be retrievable"
    );
}

#[test]
fn test_update_entry_add_protected_custom_field() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    // Create entry without protected fields
    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Entry Without Protected".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
            },
        )
        .expect("create entry");

    // Update to add protected field
    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert("NewSecret".to_string(), SecureString::from("added-value"));

    let updated = service
        .update_entry(
            &entry.id,
            UpdateEntryData {
                title: None,
                username: None,
                password: None,
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("update entry");

    assert!(
        updated
            .custom_field_meta
            .iter()
            .any(|meta| meta.key == "NewSecret" && meta.is_protected),
        "New protected field should appear in metadata"
    );

    let protected = service
        .get_entry_protected_custom_field(&entry.id, "NewSecret")
        .expect("get new protected field");

    assert_eq!(
        protected.value, "added-value",
        "Newly added protected field should be retrievable"
    );
}

#[test]
fn test_update_entry_modify_protected_custom_field() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert("ModifyMe".to_string(), SecureString::from("original-value"));

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Entry To Modify".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("create entry");

    // Verify original value
    let original = service
        .get_entry_protected_custom_field(&entry.id, "ModifyMe")
        .expect("get original protected field");
    assert_eq!(original.value, "original-value");

    // Update the protected field
    let mut updated_protected: BTreeMap<String, SecureString> = BTreeMap::new();
    updated_protected.insert("ModifyMe".to_string(), SecureString::from("updated-value"));

    service
        .update_entry(
            &entry.id,
            UpdateEntryData {
                title: None,
                username: None,
                password: None,
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(updated_protected),
            },
        )
        .expect("update entry");

    let updated = service
        .get_entry_protected_custom_field(&entry.id, "ModifyMe")
        .expect("get updated protected field");

    assert_eq!(
        updated.value, "updated-value",
        "Protected field value should be updated"
    );
}

#[test]
fn test_multiple_protected_custom_fields() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert("APIKey".to_string(), SecureString::from("api-key-value"));
    protected_custom_fields.insert("SecretToken".to_string(), SecureString::from("token-value"));
    protected_custom_fields.insert(
        "PrivateKey".to_string(),
        SecureString::from("private-key-value"),
    );

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Multiple Protected Fields".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("create entry");

    // Verify all three fields are retrievable independently
    let api_key = service
        .get_entry_protected_custom_field(&entry.id, "APIKey")
        .expect("get APIKey");
    assert_eq!(api_key.value, "api-key-value");

    let secret_token = service
        .get_entry_protected_custom_field(&entry.id, "SecretToken")
        .expect("get SecretToken");
    assert_eq!(secret_token.value, "token-value");

    let private_key = service
        .get_entry_protected_custom_field(&entry.id, "PrivateKey")
        .expect("get PrivateKey");
    assert_eq!(private_key.value, "private-key-value");

    // Verify metadata lists all three as protected
    let protected_meta: Vec<_> = entry
        .custom_field_meta
        .iter()
        .filter(|meta| meta.is_protected)
        .collect();
    assert_eq!(
        protected_meta.len(),
        3,
        "Should have 3 protected fields in metadata"
    );
}

#[test]
fn test_mixed_protected_and_unprotected_fields() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut custom_fields = BTreeMap::new();
    custom_fields.insert("Category".to_string(), "Work".to_string());
    custom_fields.insert("Website".to_string(), "example.com".to_string());

    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert("APIKey".to_string(), SecureString::from("secret-api-key"));
    protected_custom_fields.insert("PIN".to_string(), SecureString::from("1234"));

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Mixed Fields Entry".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: Some(custom_fields),
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("create entry");

    // Verify unprotected fields are in custom_fields
    assert_eq!(
        entry.custom_fields.get("Category").map(String::as_str),
        Some("Work"),
        "Unprotected field 'Category' should be in custom_fields"
    );
    assert_eq!(
        entry.custom_fields.get("Website").map(String::as_str),
        Some("example.com"),
        "Unprotected field 'Website' should be in custom_fields"
    );

    // Verify protected fields are NOT in custom_fields
    assert!(
        !entry.custom_fields.contains_key("APIKey"),
        "Protected field 'APIKey' should not be in custom_fields"
    );
    assert!(
        !entry.custom_fields.contains_key("PIN"),
        "Protected field 'PIN' should not be in custom_fields"
    );

    // Verify protected fields are retrievable via lazy decryption
    let api_key = service
        .get_entry_protected_custom_field(&entry.id, "APIKey")
        .expect("get APIKey");
    assert_eq!(api_key.value, "secret-api-key");

    let pin = service
        .get_entry_protected_custom_field(&entry.id, "PIN")
        .expect("get PIN");
    assert_eq!(pin.value, "1234");

    // Verify metadata correctly identifies protected vs unprotected
    let protected_count = entry
        .custom_field_meta
        .iter()
        .filter(|meta| meta.is_protected)
        .count();
    let unprotected_count = entry
        .custom_field_meta
        .iter()
        .filter(|meta| !meta.is_protected)
        .count();

    assert_eq!(protected_count, 2, "Should have 2 protected fields");
    assert_eq!(unprotected_count, 2, "Should have 2 unprotected fields");
}

#[test]
fn test_get_protected_field_entry_not_found() {
    let (service, _dir) = create_test_database();

    let result = service.get_entry_protected_custom_field("nonexistent-entry-id", "SomeField");

    assert!(
        matches!(result, Err(AppError::EntryNotFound(_))),
        "Should fail with EntryNotFound for invalid entry ID"
    );
}

#[test]
fn test_get_protected_field_field_not_found() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert("ExistingField".to_string(), SecureString::from("value"));

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Entry".to_string(),
                username: "user".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("create entry");

    let result = service.get_entry_protected_custom_field(&entry.id, "NonexistentField");

    assert!(
        matches!(result, Err(AppError::CustomFieldNotFound(_))),
        "Should fail with CustomFieldNotFound for missing field"
    );
}

#[test]
fn test_get_protected_field_database_not_open() {
    let service = KdbxService::new();

    let result = service.get_entry_protected_custom_field("some-id", "SomeField");

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open"
    );
}

// ============================================================================
// Password field tests
// ============================================================================

#[test]
fn test_password_roundtrip_save_reopen() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("password-roundtrip.kdbx");

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
            "Password Roundtrip",
            &options,
        )
        .expect("Failed to create test database");

    let info = service.get_info().expect("database info");

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Password Test".to_string(),
                username: "user".to_string(),
                password: SecureString::from("super-secret-password"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
            },
        )
        .expect("create entry");

    let entry_id = entry.id.clone();

    // Save and close
    service.save().expect("save database");
    let _ = service.close();

    // Reopen and verify password
    service
        .open(&db_path.to_string_lossy(), "testpass")
        .expect("reopen database");

    let password = service
        .get_entry_password(&entry_id)
        .expect("get password after reopen");

    assert_eq!(
        password, "super-secret-password",
        "Password should persist after save/reopen"
    );
}

#[test]
fn test_password_unicode() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let unicode_password = "密码🔐パスワード🗝️Contraseña";

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Unicode Password".to_string(),
                username: "user".to_string(),
                password: SecureString::from(unicode_password),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
            },
        )
        .expect("create entry");

    let retrieved_password = service
        .get_entry_password(&entry.id)
        .expect("get unicode password");

    assert_eq!(
        retrieved_password, unicode_password,
        "Unicode password should be stored and retrieved correctly"
    );
}

#[test]
fn test_password_special_characters() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let special_password = "<>&\"'{}[]|\\`~!@#$%^&*()_+-=";

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Special Chars Password".to_string(),
                username: "user".to_string(),
                password: SecureString::from(special_password),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
            },
        )
        .expect("create entry");

    let retrieved_password = service
        .get_entry_password(&entry.id)
        .expect("get special chars password");

    assert_eq!(
        retrieved_password, special_password,
        "Password with special characters should be stored and retrieved correctly"
    );
}
