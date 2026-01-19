// SPDX-License-Identifier: MIT
//! Tests for Tauri command handlers
//!
//! Since Tauri commands require a runtime with managed state (`State<'_, Arc<T>>`),
//! these tests exercise the underlying service methods that commands delegate to.
//! This test structure is ready for when command-specific logic is added.

#[path = "support/mod.rs"]
mod support;

use mithril_vault_lib::services::kdbx::KdbxService;
use support::fixture_path;

fn fixture_exists(filename: &str) -> bool {
    fixture_path(filename).exists()
}

fn open_test_database() -> Option<KdbxService> {
    if !fixture_exists("test-kdbx4-low-KDF.kdbx") {
        return None;
    }

    let service = KdbxService::new();
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    let result = service.open(&path.to_string_lossy(), "test123");
    assert!(result.is_ok(), "Failed to open test database");
    Some(service)
}

#[path = "commands/database_test.rs"]
mod database_test;

#[path = "commands/entries_test.rs"]
mod entries_test;

#[path = "commands/groups_test.rs"]
mod groups_test;
