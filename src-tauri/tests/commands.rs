// SPDX-License-Identifier: MIT
//! Tests for Tauri command handlers
//!
//! Since Tauri commands require a runtime with managed state (`State<'_, Arc<T>>`),
//! these tests exercise the underlying service methods that commands delegate to.
//! This test structure is ready for when command-specific logic is added.

#[path = "commands/database_test.rs"]
mod database_test;

#[path = "commands/entries_test.rs"]
mod entries_test;

#[path = "commands/groups_test.rs"]
mod groups_test;
