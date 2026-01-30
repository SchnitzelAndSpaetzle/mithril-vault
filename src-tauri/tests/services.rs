// SPDX-License-Identifier: MIT
//! Tests for backend services.

use std::sync::{Mutex, MutexGuard};

// Tauri's test app uses a shared app-local data directory within a test process.
// These settings tests perform filesystem operations on the same settings.json path,
// so we serialize them to avoid cross-test races.
static SETTINGS_TEST_LOCK: Mutex<()> = Mutex::new(());

fn settings_test_lock() -> MutexGuard<'static, ()> {
    SETTINGS_TEST_LOCK.lock().expect("settings test lock")
}

#[path = "services/settings_service_test.rs"]
mod settings_service_test;

#[path = "services/settings_service_unitlike_test.rs"]
mod settings_service_unitlike_test;
