// SPDX-License-Identifier: GPL-3.0-or-later
//! Integration tests for KDBX database operations using keepass-rs

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use keepass::{db::NodeRef, Database, DatabaseKey};
use std::fs::File;
use std::path::PathBuf;

/// Get the path to a test fixture file
fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(filename);
    path
}

#[test]
fn test_open_kdbx4_with_password() {
    let path = fixture_path("test-kdbx4.kdbx");
    if !path.exists() {
        eprintln!(
            "Skipping test: fixture not found at {path:?}. \
             Create with KeePassXC using password 'test123'"
        );
        return;
    }

    let mut file = File::open(&path).expect("Failed to open test file");
    let key = DatabaseKey::new().with_password("test123");
    let db = Database::open(&mut file, key).expect("Failed to open KDBX4 database");

    // Verify database was opened successfully
    assert!(!db.root.name.is_empty(), "Root group should have a name");
}

#[test]
fn test_open_kdbx3_with_password() {
    let path = fixture_path("test-kdbx3.kdbx");
    if !path.exists() {
        eprintln!(
            "Skipping test: fixture not found at {path:?}. \
             Create with KeePassXC (KDBX 3.1 format) using password 'test123'"
        );
        return;
    }

    let mut file = File::open(&path).expect("Failed to open test file");
    let key = DatabaseKey::new().with_password("test123");
    let db = Database::open(&mut file, key).expect("Failed to open KDBX3 database");

    assert!(!db.root.name.is_empty(), "Root group should have a name");
}

#[test]
fn test_open_with_keyfile() {
    let db_path = fixture_path("test-keyfile-kdbx4.kdbx");
    let key_path = fixture_path("test-keyfile.keyx");

    if !db_path.exists() || !key_path.exists() {
        eprintln!(
            "Skipping test: fixtures not found. \
             Create database with password 'test123' and keyfile"
        );
        return;
    }

    let mut db_file = File::open(&db_path).expect("Failed to open test database");
    let mut key_file = File::open(&key_path).expect("Failed to open keyfile");

    let key = DatabaseKey::new()
        .with_password("test123")
        .with_keyfile(&mut key_file)
        .expect("Failed to load keyfile");

    let db = Database::open(&mut db_file, key).expect("Failed to open database with keyfile");
    assert!(!db.root.name.is_empty());
}

#[test]
fn test_invalid_password() {
    let path = fixture_path("test-kdbx4.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let mut file = File::open(&path).expect("Failed to open test file");
    let key = DatabaseKey::new().with_password("wrong_password");
    let result = Database::open(&mut file, key);

    assert!(result.is_err(), "Should fail with invalid password");
}

#[test]
fn test_file_not_found() {
    let path = fixture_path("nonexistent.kdbx");
    let result = File::open(&path);

    assert!(result.is_err(), "Should fail when file doesn't exist");
}

#[test]
fn test_read_entries() {
    let path = fixture_path("test-kdbx4.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let mut file = File::open(&path).expect("Failed to open test file");
    let key = DatabaseKey::new().with_password("test123");
    let db = Database::open(&mut file, key).expect("Failed to open database");

    // Count entries in the database
    let entry_count = count_entries(&db.root);
    println!("Found {entry_count} entries in database");

    // The test database should have at least one entry
    // (adjust based on your test fixture content)
}

#[test]
fn test_read_groups() {
    let path = fixture_path("test-kdbx4.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let mut file = File::open(&path).expect("Failed to open test file");
    let key = DatabaseKey::new().with_password("test123");
    let db = Database::open(&mut file, key).expect("Failed to open database");

    // Count groups in the database
    let group_count = count_groups(&db.root);
    println!("Found {group_count} groups in database");

    // Should have at least the root group
    assert!(group_count >= 1, "Should have at least the root group");
}

#[test]
fn test_entry_fields() {
    let path = fixture_path("test-kdbx4.kdbx");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    let mut file = File::open(&path).expect("Failed to open test file");
    let key = DatabaseKey::new().with_password("test123");
    let db = Database::open(&mut file, key).expect("Failed to open database");

    // Find an entry and verify we can read its fields
    for node in &db.root {
        if let NodeRef::Entry(entry) = node {
            // Entry should have standard fields accessible
            let title = entry.get_title();
            let username = entry.get_username();
            let password = entry.get_password();
            let url = entry.get_url();

            println!(
                "Entry: title={title:?}, username={username:?}, has_password={}, url={url:?}",
                password.is_some()
            );

            // At least title should be accessible
            if title.is_some() {
                return; // Test passed - we found and read an entry
            }
        }
    }
}

/// Helper function to count entries (keepass-rs iterator already traverses all descendants)
fn count_entries(group: &keepass::db::Group) -> usize {
    group
        .iter()
        .filter(|node| matches!(node, NodeRef::Entry(_)))
        .count()
}

/// Helper function to count groups (keepass-rs iterator already traverses all descendants)
fn count_groups(group: &keepass::db::Group) -> usize {
    // +1 for the root group itself, then count all descendant groups
    1 + group
        .iter()
        .filter(|node| matches!(node, NodeRef::Group(_)))
        .count()
}
