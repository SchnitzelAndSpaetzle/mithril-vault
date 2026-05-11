// SPDX-License-Identifier: MIT
//! Tests for settings command handlers

#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::settings::{
    add_recent_database, clear_recent_databases, get_app_preferences, get_keyfile_for_database,
    get_settings, remove_recent_database, reset_app_preferences, update_app_preferences,
    update_settings, StartupBehavior,
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
    assert!(settings.prevent_screen_capture);

    let mut updated = settings.clone();
    updated.auto_lock_timeout = 90;
    updated.theme = "light".into();
    updated.prevent_screen_capture = false;

    tauri::async_runtime::block_on(update_settings(updated, app.state())).expect("update settings");

    let refreshed =
        tauri::async_runtime::block_on(get_settings(app.state())).expect("get settings");
    assert_eq!(refreshed.auto_lock_timeout, 90);
    assert_eq!(refreshed.theme, "light");
    assert!(!refreshed.prevent_screen_capture);

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

#[test]
fn app_preferences_commands() {
    let app = setup_app();

    tauri::async_runtime::block_on(add_recent_database(
        "db-1.kdbx".into(),
        Some("key-1.key".into()),
        app.state(),
    ))
    .expect("add recent database");

    let mut prefs =
        tauri::async_runtime::block_on(get_app_preferences(app.state())).expect("get preferences");
    assert_eq!(prefs.security.clipboard_clear_timeout, 30);
    assert!(!prefs.security.auto_download_favicons);
    assert!(!prefs.security.allow_third_party_favicon_fallbacks);
    assert_eq!(
        prefs.general.startup_behavior,
        StartupBehavior::ShowUnlockScreen
    );
    assert!(!prefs.advanced.data_location.is_empty());

    prefs.general.language = "de".into();
    prefs.general.startup_behavior = StartupBehavior::OpenLastDatabase;
    prefs.security.clipboard_clear_timeout = 12;
    prefs.security.auto_download_favicons = true;
    prefs.security.allow_third_party_favicon_fallbacks = true;
    prefs.appearance.theme = "light".into();
    prefs.browser_integration.enabled = true;
    prefs.browser_integration.allowed_sites = vec!["example.com".into()];
    prefs.advanced.debug_mode = true;

    tauri::async_runtime::block_on(update_app_preferences(prefs.clone(), app.state()))
        .expect("update preferences");

    let refreshed =
        tauri::async_runtime::block_on(get_app_preferences(app.state())).expect("get preferences");
    assert_eq!(refreshed.general.language, "de");
    assert_eq!(
        refreshed.general.startup_behavior,
        StartupBehavior::OpenLastDatabase
    );
    assert_eq!(refreshed.security.clipboard_clear_timeout, 12);
    assert!(refreshed.security.auto_download_favicons);
    assert!(refreshed.security.allow_third_party_favicon_fallbacks);
    assert_eq!(refreshed.appearance.theme, "light");
    assert!(refreshed.browser_integration.enabled);
    assert_eq!(
        refreshed.browser_integration.allowed_sites,
        vec!["example.com"]
    );
    assert!(refreshed.advanced.debug_mode);

    let reset =
        tauri::async_runtime::block_on(reset_app_preferences(app.state())).expect("reset prefs");
    assert_eq!(reset.general.language, "en");
    assert_eq!(reset.security.clipboard_clear_timeout, 30);
    assert!(!reset.security.auto_download_favicons);
    assert!(!reset.security.allow_third_party_favicon_fallbacks);
    assert_eq!(reset.appearance.theme, "system");
    assert!(!reset.advanced.debug_mode);

    let settings = tauri::async_runtime::block_on(get_settings(app.state())).expect("get settings");
    assert_eq!(settings.recent_databases.len(), 1);

    cleanup_settings_file(&app);
}
