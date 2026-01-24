// SPDX-License-Identifier: MIT
//! Integration tests for secure memory handling with zeroization.
//!
//! These tests verify that passwords and sensitive data are properly handled
//! using SecureString/SecureBytes types that automatically zeroize on drop.

#![allow(clippy::expect_used)]

use mithril_vault_lib::domain::secure::SecureString;
use mithril_vault_lib::dto::database::DatabaseCreationOptions;
use mithril_vault_lib::dto::entry::CreateEntryData;
use mithril_vault_lib::services::kdbx::KdbxService;
use std::collections::BTreeMap;

/// Helper to create a test database with minimal KDF settings for fast tests
fn create_test_database() -> (KdbxService, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("secure-memory-test.kdbx");

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
            Some("test-secure-password"),
            None,
            "Secure Memory Test",
            &options,
        )
        .expect("Failed to create test database");

    (service, dir)
}

#[test]
fn test_create_database_with_secure_password() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("secure-create.kdbx");

    let options = DatabaseCreationOptions {
        create_default_groups: false,
        kdf_memory: Some(1024 * 1024),
        kdf_iterations: Some(1),
        kdf_parallelism: Some(1),
        description: None,
    };

    let service = KdbxService::new();

    // Create database with password
    let result = service.create_database(
        &db_path.to_string_lossy(),
        Some("secure-password-123"),
        None,
        "Secure Database",
        &options,
    );

    assert!(result.is_ok(), "Database creation should succeed");

    // Close and reopen with the same password
    let _ = service.close();

    let reopen_result = service.open(&db_path.to_string_lossy(), "secure-password-123");
    assert!(
        reopen_result.is_ok(),
        "Should be able to reopen with correct password"
    );
}

#[test]
fn test_save_reopen_with_secure_password() {
    let (service, dir) = create_test_database();
    let db_path = dir.path().join("secure-memory-test.kdbx");
    let info = service.get_info().expect("database info");

    // Create an entry with a secure password
    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "Secure Entry".to_string(),
                username: "user".to_string(),
                password: SecureString::from("entry-secure-password"),
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

    // Save the database
    service.save().expect("save database");

    // Close and reopen
    let _ = service.close();
    service
        .open(&db_path.to_string_lossy(), "test-secure-password")
        .expect("reopen database");

    // Verify the entry password persisted correctly
    let password = service.get_entry_password(&entry_id).expect("get password");
    assert_eq!(
        password, "entry-secure-password",
        "Entry password should persist after save/reopen"
    );
}

#[test]
fn test_secure_string_debug_does_not_leak() {
    let secure = SecureString::from("super-secret-password");

    // Debug output should not contain the actual password
    let debug_output = format!("{secure:?}");
    assert!(
        !debug_output.contains("super-secret-password"),
        "Debug output should not contain the password"
    );
    assert!(
        debug_output.contains("[REDACTED]"),
        "Debug output should show [REDACTED]"
    );
}

#[test]
fn test_secure_string_display_does_not_leak() {
    let secure = SecureString::from("super-secret-password");

    // Display output should not contain the actual password
    let display_output = format!("{secure}");
    assert!(
        !display_output.contains("super-secret-password"),
        "Display output should not contain the password"
    );
    assert_eq!(
        display_output, "[REDACTED]",
        "Display output should be [REDACTED]"
    );
}

#[test]
fn test_protected_custom_fields_with_secure_string() {
    let (service, _dir) = create_test_database();
    let info = service.get_info().expect("database info");

    let mut protected_fields: BTreeMap<String, SecureString> = BTreeMap::new();
    protected_fields.insert("APIKey".to_string(), SecureString::from("secret-api-key"));
    protected_fields.insert("Token".to_string(), SecureString::from("secret-token"));

    let entry = service
        .create_entry(
            &info.root_group_id,
            CreateEntryData {
                title: "API Entry".to_string(),
                username: "api-user".to_string(),
                password: SecureString::from("api-password"),
                url: None,
                notes: None,
                icon_id: None,
                tags: None,
                custom_fields: None,
                protected_custom_fields: Some(protected_fields),
            },
        )
        .expect("create entry");

    // Verify protected fields are retrievable
    let api_key = service
        .get_entry_protected_custom_field(&entry.id, "APIKey")
        .expect("get APIKey");
    assert_eq!(api_key.value, "secret-api-key");

    let token = service
        .get_entry_protected_custom_field(&entry.id, "Token")
        .expect("get Token");
    assert_eq!(token.value, "secret-token");
}
