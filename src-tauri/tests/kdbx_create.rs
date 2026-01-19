#![allow(clippy::expect_used)]

use mithril_vault_lib::dto::database::DatabaseCreationOptions;
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use tempfile::tempdir;

#[path = "support/mod.rs"]
mod support;

use support::fixture_path;

#[test]
fn test_create_new_database() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("new-database.kdbx");

    let service = KdbxService::new();
    let info = service
        .create(&db_path.to_string_lossy(), "testpass123", "My New Vault")
        .expect("Failed to create database");

    assert_eq!(info.name, "My New Vault");
    assert!(!info.root_group_id.is_empty());
    assert!(!info.is_modified);

    assert!(db_path.exists(), "Database file should exist");

    service.close().expect("Failed to close");

    let reopened_info = service
        .open(&db_path.to_string_lossy(), "testpass123")
        .expect("Failed to reopen database");

    assert_eq!(reopened_info.name, "My New Vault");
}

#[test]
fn test_create_fails_when_database_already_open() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path1 = dir.path().join("db1.kdbx");
    let db_path2 = dir.path().join("db2.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path1.to_string_lossy(), "pass1", "DB1")
        .expect("Failed to create first database");

    let result = service.create(&db_path2.to_string_lossy(), "pass2", "DB2");
    assert!(
        matches!(result, Err(AppError::DatabaseAlreadyOpen)),
        "Should not allow creating when database is already open"
    );
}

#[test]
fn test_create_database_with_default_options() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("default-options.kdbx");

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword123"),
            None,
            "Default Options DB",
            &DatabaseCreationOptions::default(),
        )
        .expect("Failed to create database with default options");

    assert_eq!(info.name, "Default Options DB");
    assert_eq!(info.version, "KDBX 4.0");
    assert!(!info.root_group_id.is_empty());

    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpassword123")
        .expect("Failed to reopen database");
}

#[test]
fn test_create_database_with_keyfile() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-create.kdbx");
    let keyfile_path = fixture_path("test-keyfile.keyx");

    if !keyfile_path.exists() {
        eprintln!("Skipping test: keyfile fixture not found");
        return;
    }

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            Some(&keyfile_path.to_string_lossy()),
            "Keyfile DB",
            &DatabaseCreationOptions::default(),
        )
        .expect("Failed to create database with keyfile");

    assert_eq!(info.name, "Keyfile DB");
    assert_eq!(info.version, "KDBX 4.0");

    service.close().expect("Failed to close");

    let result = service.open(&db_path.to_string_lossy(), "testpassword");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Should require keyfile to open: got {result:?}"
    );

    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "testpassword",
            &keyfile_path.to_string_lossy(),
        )
        .expect("Should open with password + keyfile");
}

#[test]
fn test_create_database_with_keyfile_only() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-only-create.kdbx");
    let keyfile_path = fixture_path("test-keyfile.keyx");

    if !keyfile_path.exists() {
        eprintln!("Skipping test: keyfile fixture not found");
        return;
    }

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            None,
            Some(&keyfile_path.to_string_lossy()),
            "Keyfile Only DB",
            &DatabaseCreationOptions::default(),
        )
        .expect("Failed to create database with keyfile only");

    assert_eq!(info.name, "Keyfile Only DB");

    service.close().expect("Failed to close");

    service
        .open_with_keyfile_only(&db_path.to_string_lossy(), &keyfile_path.to_string_lossy())
        .expect("Should open with keyfile only");
}

#[test]
fn test_create_database_fails_without_credentials() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("no-creds.kdbx");

    let service = KdbxService::new();
    let result = service.create_database(
        &db_path.to_string_lossy(),
        None,
        None,
        "No Credentials DB",
        &DatabaseCreationOptions::default(),
    );

    assert!(
        matches!(result, Err(AppError::NoCredentials)),
        "Should fail without credentials: got {result:?}"
    );
}

#[test]
fn test_create_database_with_default_groups() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("default-groups.kdbx");

    let options = DatabaseCreationOptions {
        create_default_groups: true,
        ..Default::default()
    };

    let service = KdbxService::new();
    service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            None,
            "Default Groups DB",
            &options,
        )
        .expect("Failed to create database with default groups");

    let groups = service.list_groups().expect("Failed to list groups");

    assert!(!groups.is_empty(), "Should have at least root group");

    let root = &groups[0];
    assert_eq!(root.name, "Default Groups DB");

    assert_eq!(root.children.len(), 4, "Root should have 4 default groups");

    let child_names: Vec<&str> = root.children.iter().map(|g| g.name.as_str()).collect();
    assert!(
        child_names.contains(&"General"),
        "Should have General group"
    );
    assert!(child_names.contains(&"Email"), "Should have Email group");
    assert!(
        child_names.contains(&"Banking"),
        "Should have Banking group"
    );
    assert!(child_names.contains(&"Social"), "Should have Social group");
}

#[test]
fn test_create_database_without_default_groups() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("no-default-groups.kdbx");

    let options = DatabaseCreationOptions {
        create_default_groups: false,
        ..Default::default()
    };

    let service = KdbxService::new();
    service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            None,
            "No Default Groups DB",
            &options,
        )
        .expect("Failed to create database");

    let groups = service.list_groups().expect("Failed to list groups");
    let root = &groups[0];

    assert!(
        root.children.is_empty(),
        "Root should have no child groups when create_default_groups is false"
    );
}

#[test]
fn test_create_database_with_description() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("with-description.kdbx");

    let options = DatabaseCreationOptions {
        description: Some("This is my test database description".to_string()),
        ..Default::default()
    };

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            None,
            "Description DB",
            &options,
        )
        .expect("Failed to create database with description");

    assert_eq!(info.name, "Description DB");

    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpassword")
        .expect("Failed to reopen");
}

#[test]
fn test_create_database_with_custom_kdf_settings() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("custom-kdf.kdbx");

    let options = DatabaseCreationOptions {
        kdf_memory: Some(16 * 1024 * 1024),
        kdf_iterations: Some(2),
        kdf_parallelism: Some(2),
        ..Default::default()
    };

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            None,
            "Custom KDF DB",
            &options,
        )
        .expect("Failed to create database with custom KDF");

    assert_eq!(info.version, "KDBX 4.0");

    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpassword")
        .expect("Failed to reopen database with custom KDF");
}

#[test]
fn test_create_database_with_all_options() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("all-options.kdbx");
    let keyfile_path = fixture_path("test-keyfile.keyx");

    if !keyfile_path.exists() {
        eprintln!("Skipping test: keyfile fixture not found");
        return;
    }

    let options = DatabaseCreationOptions {
        description: Some("Full featured database".to_string()),
        create_default_groups: true,
        kdf_memory: Some(32 * 1024 * 1024),
        kdf_iterations: Some(2),
        kdf_parallelism: Some(2),
    };

    let service = KdbxService::new();
    let info = service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            Some(&keyfile_path.to_string_lossy()),
            "Full Featured DB",
            &options,
        )
        .expect("Failed to create database with all options");

    assert_eq!(info.name, "Full Featured DB");
    assert_eq!(info.version, "KDBX 4.0");

    let groups = service.list_groups().expect("Failed to list groups");
    let root = &groups[0];
    assert_eq!(root.children.len(), 4, "Should have 4 default groups");

    service.close().expect("Failed to close");
    service
        .open_with_keyfile(
            &db_path.to_string_lossy(),
            "testpassword",
            &keyfile_path.to_string_lossy(),
        )
        .expect("Failed to reopen with all options");
}

#[test]
fn test_create_database_legacy_api_still_works() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("legacy-api.kdbx");

    let service = KdbxService::new();
    let info = service
        .create(&db_path.to_string_lossy(), "testpassword", "Legacy DB")
        .expect("Legacy create() should still work");

    assert_eq!(info.name, "Legacy DB");
    assert_eq!(info.version, "KDBX 4.0");

    service.close().expect("Failed to close");
    service
        .open(&db_path.to_string_lossy(), "testpassword")
        .expect("Legacy created DB should reopen");
}

#[test]
fn test_database_creation_options_defaults() {
    let options = DatabaseCreationOptions::default();

    assert_eq!(
        options.memory_bytes(),
        64 * 1024 * 1024,
        "Default memory should be 64 MB"
    );
    assert_eq!(options.iterations(), 3, "Default iterations should be 3");
    assert_eq!(options.parallelism(), 4, "Default parallelism should be 4");
    assert!(
        !options.create_default_groups,
        "Default should not create groups"
    );
    assert!(
        options.description.is_none(),
        "Default should have no description"
    );
}

#[test]
fn test_database_creation_options_custom_values() {
    let options = DatabaseCreationOptions {
        kdf_memory: Some(128 * 1024 * 1024),
        kdf_iterations: Some(5),
        kdf_parallelism: Some(8),
        description: Some("Test".to_string()),
        create_default_groups: true,
    };

    assert_eq!(options.memory_bytes(), 128 * 1024 * 1024);
    assert_eq!(options.iterations(), 5);
    assert_eq!(options.parallelism(), 8);
}
