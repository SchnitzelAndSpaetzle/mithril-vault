#![allow(clippy::expect_used)]

use mithril_vault_lib::domain::secure::SecureString;
use mithril_vault_lib::dto::database::DatabaseCreationOptions;
use mithril_vault_lib::dto::entry::CreateEntryData;
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use std::collections::BTreeMap;
use tempfile::TempDir;

#[path = "support/mod.rs"]
mod support;

use support::fixture_path;

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
fn test_list_entries_without_open() {
    let service = KdbxService::new();
    let result = service.list_entries(None);
    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should error when listing entries without an open database"
    );
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

// ============================================================================
// Protected field integration tests
// ============================================================================

/// Helper to create a KDBX4 test database
fn create_kdbx4_database() -> (KdbxService, TempDir) {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("protected-fields.kdbx");

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
            "Protected Fields Test",
            &options,
        )
        .expect("Failed to create test database");

    (service, dir)
}

#[test]
fn test_protected_fields_kdbx4_roundtrip() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("kdbx4-protected.kdbx");

    let options = DatabaseCreationOptions {
        create_default_groups: false,
        kdf_memory: Some(1024 * 1024),
        kdf_iterations: Some(1),
        kdf_parallelism: Some(1),
        description: None,
    };

    // Create KDBX4 database with protected fields
    let service = KdbxService::new();
    service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpass"),
            None,
            "KDBX4 Protected Test",
            &options,
        )
        .expect("Failed to create database");

    let info = service.get_info().expect("database info");

    let mut custom_fields = BTreeMap::new();
    custom_fields.insert("Category".to_string(), "Test".to_string());

    let mut protected_custom_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_custom_fields.insert(
        "APIKey".to_string(),
        SecureString::from("secret-api-key-12345"),
    );
    protected_custom_fields.insert(
        "SecretToken".to_string(),
        SecureString::from("bearer-token-xyz"),
    );

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Integration Test Entry".to_string(),
                username: "testuser".to_string(),
                password: SecureString::from("integration-password"),
                url: Some("https://example.com".to_string()),
                notes: Some("Test notes".to_string()),
                icon_id: None,
                tags: None,
                custom_fields: Some(custom_fields),
                protected_custom_fields: Some(protected_custom_fields),
            },
        )
        .expect("create entry");

    let entry_id = entry.id.clone();

    // Save and close
    service.save().expect("save database");
    let _ = service.close();

    // Reopen and verify all protected fields
    service
        .open(&db_path.to_string_lossy(), "testpass")
        .expect("reopen database");

    // Verify entry exists
    let reopened_entry = service
        .get_entry(&entry_id)
        .expect("get entry after reopen");
    assert_eq!(reopened_entry.title, "Integration Test Entry");

    // Verify unprotected custom field
    assert_eq!(
        reopened_entry
            .custom_fields
            .get("Category")
            .map(String::as_str),
        Some("Test"),
        "Unprotected custom field should persist"
    );

    // Verify password
    let password = service.get_entry_password(&entry_id).expect("get password");
    assert_eq!(
        password, "integration-password",
        "Password should persist in KDBX4"
    );

    // Verify protected custom fields
    let api_key = service
        .get_entry_protected_custom_field(&entry_id, "APIKey")
        .expect("get APIKey");
    assert_eq!(
        api_key.value, "secret-api-key-12345",
        "Protected APIKey should persist in KDBX4"
    );

    let secret_token = service
        .get_entry_protected_custom_field(&entry_id, "SecretToken")
        .expect("get SecretToken");
    assert_eq!(
        secret_token.value, "bearer-token-xyz",
        "Protected SecretToken should persist in KDBX4"
    );

    // Verify metadata correctly identifies protected fields
    let protected_meta: Vec<_> = reopened_entry
        .custom_field_meta
        .iter()
        .filter(|meta| meta.is_protected)
        .collect();
    assert_eq!(
        protected_meta.len(),
        2,
        "Should have 2 protected fields in metadata after reopen"
    );
}

#[test]
fn test_protected_fields_persist_after_save() {
    let (service, dir) = create_kdbx4_database();
    let db_path = dir.path().join("protected-fields.kdbx");
    let info = service.get_info().expect("database info");

    // Create first entry with protected field
    let mut protected1: BTreeMap<String, SecureString> = BTreeMap::new();
    protected1.insert("Secret1".to_string(), SecureString::from("value1"));

    let entry1 = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Entry 1".to_string(),
                username: "user1".to_string(),
                password: SecureString::from("pass1"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected1),
            },
        )
        .expect("create entry 1");

    // Save database
    service.save().expect("first save");

    // Create second entry with protected field (after first save)
    let mut protected2: BTreeMap<String, SecureString> = BTreeMap::new();
    protected2.insert("Secret2".to_string(), SecureString::from("value2"));

    let entry2 = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Entry 2".to_string(),
                username: "user2".to_string(),
                password: SecureString::from("pass2"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected2),
            },
        )
        .expect("create entry 2");

    // Save again
    service.save().expect("second save");

    let entry1_id = entry1.id.clone();
    let entry2_id = entry2.id.clone();

    // Close and reopen
    let _ = service.close();
    service
        .open(&db_path.to_string_lossy(), "testpass")
        .expect("reopen database");

    // Verify both entries and their protected fields
    let secret1 = service
        .get_entry_protected_custom_field(&entry1_id, "Secret1")
        .expect("get Secret1");
    assert_eq!(
        secret1.value, "value1",
        "First entry's protected field should persist after multiple saves"
    );

    let secret2 = service
        .get_entry_protected_custom_field(&entry2_id, "Secret2")
        .expect("get Secret2");
    assert_eq!(
        secret2.value, "value2",
        "Second entry's protected field should persist after save"
    );

    // Verify passwords also persisted
    let pass1 = service.get_entry_password(&entry1_id).expect("get pass1");
    assert_eq!(pass1, "pass1");

    let pass2 = service.get_entry_password(&entry2_id).expect("get pass2");
    assert_eq!(pass2, "pass2");
}
