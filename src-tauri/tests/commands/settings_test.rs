// SPDX-License-Identifier: MIT
//! Tests for settings command handlers

#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::settings::{
    add_recent_database, clear_recent_databases, get_keyfile_for_database, get_settings,
    remove_recent_database, update_settings,
};
use mithril_vault_lib::services::settings::SettingsService;
use std::sync::Arc;
use tauri::test::mock_app;
use tauri::Manager;

fn setup_app() -> tauri::App<tauri::test::MockRuntime> {
    let app = mock_app();
    let settings_service = SettingsService::new(app.handle()).expect("create settings service");
    app.manage(Arc::new(settings_service));
    app
}

fn cleanup_settings_file(app: &tauri::App<tauri::test::MockRuntime>) {
    if let Ok(data_dir) = app.path().app_local_data_dir() {
        let settings_path = data_dir.join("settings.json");
        if settings_path.exists() {
            let _ = std::fs::remove_file(settings_path);
        }
    }
}

#[test]
fn get_and_update_settings_commands() {
    let app = setup_app();

    let settings = tauri::async_runtime::block_on(get_settings(app.state())).expect("get settings");
    assert_eq!(settings.auto_lock_timeout, 300);

    let mut updated = settings.clone();
    updated.auto_lock_timeout = 90;
    updated.theme = "light".into();

    tauri::async_runtime::block_on(update_settings(updated, app.state())).expect("update settings");

    let refreshed =
        tauri::async_runtime::block_on(get_settings(app.state())).expect("get settings");
    assert_eq!(refreshed.auto_lock_timeout, 90);
    assert_eq!(refreshed.theme, "light");

    cleanup_settings_file(&app);
}

#[test]
fn recent_database_commands() {
    let app = setup_app();

    tauri::async_runtime::block_on(add_recent_database(
        "db-1.kdbx".into(),
        Some("key-1.key".into()),
        app.state(),
    ))
    .expect("add recent database");

    let keyfile =
        tauri::async_runtime::block_on(get_keyfile_for_database("db-1.kdbx".into(), app.state()))
            .expect("get keyfile");
    assert_eq!(keyfile.as_deref(), Some("key-1.key"));

    tauri::async_runtime::block_on(remove_recent_database("db-1.kdbx".into(), app.state()))
        .expect("remove recent database");

    let settings = tauri::async_runtime::block_on(get_settings(app.state())).expect("get settings");
    assert!(settings.recent_databases.is_empty());

    tauri::async_runtime::block_on(add_recent_database("db-2.kdbx".into(), None, app.state()))
        .expect("add recent database");

    tauri::async_runtime::block_on(clear_recent_databases(app.state()))
        .expect("clear recent databases");

    let settings = tauri::async_runtime::block_on(get_settings(app.state())).expect("get settings");
    assert!(settings.recent_databases.is_empty());

    cleanup_settings_file(&app);
}
