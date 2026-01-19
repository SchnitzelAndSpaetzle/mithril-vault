#![allow(clippy::expect_used)]

use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;

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
