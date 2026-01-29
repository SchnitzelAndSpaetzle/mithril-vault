// SPDX-License-Identifier: MIT
//! Tests for Tauri app setup wiring

#![allow(clippy::expect_used)]

use mithril_vault_lib::services::settings::SettingsService;
use mithril_vault_lib::{build_app, register_services};
use std::sync::Arc;
use tauri::test::mock_builder;
use tauri::Manager;

fn cleanup_settings_file(app: &tauri::App<tauri::test::MockRuntime>) {
    if let Ok(data_dir) = app.path().app_local_data_dir() {
        let settings_path = data_dir.join("settings.json");
        if settings_path.exists() {
            let _ = std::fs::remove_file(settings_path);
        }
    }
}

#[test]
fn build_app_configures_builder() {
    let _builder = build_app(mock_builder());
}

#[test]
fn register_services_registers_state() {
    let app = tauri::test::mock_app();

    register_services(app.handle()).expect("register services");

    let settings_state: tauri::State<'_, Arc<SettingsService>> = app.state();
    assert!(
        settings_state.get_settings().is_ok(),
        "Settings service should be available"
    );

    cleanup_settings_file(&app);
}
