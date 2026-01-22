// SPDX-License-Identifier: MIT
//! Tests for Tauri command handlers
//!
//! Since Tauri commands require a runtime with managed state (`State<'_, Arc<T>>`),
//! these tests exercise the underlying service methods that commands delegate to.
//! This test structure is ready for when command-specific logic is added.

#[path = "support/mod.rs"]
mod support;

use mithril_vault_lib::services::kdbx::KdbxService;
use std::path::PathBuf;
use support::fixture_path;
use tempfile::TempDir;

fn fixture_exists(filename: &str) -> bool {
    fixture_path(filename).exists()
}

/// Creates a temporary copy of a fixture file for isolated testing.
/// This is necessary because file locking prevents multiple tests from
/// opening the same database file concurrently.
fn copy_fixture_to_temp(filename: &str) -> Option<(TempDir, PathBuf)> {
    if !fixture_exists(filename) {
        return None;
    }

    let source = fixture_path(filename);
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let dest = temp_dir.path().join(filename);

    std::fs::copy(&source, &dest).expect("Failed to copy fixture");
    Some((temp_dir, dest))
}

fn open_test_database() -> Option<(KdbxService, TempDir)> {
    let (temp_dir, db_path) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx")?;

    let service = KdbxService::new();
    let result = service.open(&db_path.to_string_lossy(), "test123");
    assert!(result.is_ok(), "Failed to open test database");
    Some((service, temp_dir))
}

#[path = "commands/database_test.rs"]
mod database_test;

#[path = "commands/entries_test.rs"]
mod entries_test;

#[path = "commands/groups_test.rs"]
mod groups_test;
