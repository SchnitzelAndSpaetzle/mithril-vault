// SPDX-License-Identifier: MIT
//! Tests for settings service persistence

#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::settings::AppSettings;
use mithril_vault_lib::services::settings::SettingsService;
use tauri::test::mock_app;
use tauri::Manager;

const MAX_RECENT_DATABASES: usize = 10;

fn setup_app() -> tauri::App<tauri::test::MockRuntime> {
    mock_app()
}

fn settings_path(app: &tauri::App<tauri::test::MockRuntime>) -> std::path::PathBuf {
    app.path()
        .app_local_data_dir()
        .expect("app data dir")
        .join("settings.json")
}

fn cleanup_settings_file(app: &tauri::App<tauri::test::MockRuntime>) {
    let path = settings_path(app);
    let _ = std::fs::remove_file(path);
    // Some tests intentionally create a directory at settings.json to force IO errors.
    // Clean it up as well so other tests can create the service.
    let _ = std::fs::remove_dir_all(settings_path(app));
}

#[test]
fn load_or_default_returns_default_when_missing() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    cleanup_settings_file(&app);

    let service = SettingsService::new(app.handle()).expect("create service");
    let settings = service.get_settings().expect("get settings");

    assert_eq!(settings.auto_lock_timeout, 300);
    assert_eq!(settings.clipboard_clear_timeout, 30);
    assert_eq!(settings.theme, "system");
    assert!(settings.recent_databases.is_empty());

    cleanup_settings_file(&app);
}

#[test]
fn save_and_load_roundtrip() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    cleanup_settings_file(&app);
    let service = SettingsService::new(app.handle()).expect("create service");

    let settings = AppSettings {
        auto_lock_timeout: 120,
        clipboard_clear_timeout: 15,
        show_password_by_default: true,
        minimize_to_tray: false,
        start_minimized: true,
        theme: "dark".into(),
        recent_databases: Vec::new(),
    };

    service.update_settings(settings).expect("save settings");
    let reloaded = SettingsService::new(app.handle()).expect("create service");
    let loaded = reloaded.get_settings().expect("get settings");

    assert_eq!(loaded.auto_lock_timeout, 120);
    assert_eq!(loaded.clipboard_clear_timeout, 15);
    assert!(loaded.show_password_by_default);
    assert!(!loaded.minimize_to_tray);
    assert!(loaded.start_minimized);
    assert_eq!(loaded.theme, "dark");
    assert!(loaded.recent_databases.is_empty());

    cleanup_settings_file(&app);
}

#[test]
fn update_and_get_settings_persist() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    cleanup_settings_file(&app);
    let service = SettingsService::new(app.handle()).expect("create service");

    let updated = AppSettings {
        auto_lock_timeout: 45,
        theme: "light".into(),
        ..AppSettings::default()
    };

    service
        .update_settings(updated.clone())
        .expect("update settings");

    let fetched = service.get_settings().expect("get settings");
    assert_eq!(fetched.auto_lock_timeout, 45);
    assert_eq!(fetched.theme, "light");

    let reloaded = SettingsService::new(app.handle()).expect("create service");
    let loaded = reloaded.get_settings().expect("get settings");
    assert_eq!(loaded.auto_lock_timeout, 45);
    assert_eq!(loaded.theme, "light");

    cleanup_settings_file(&app);
}

#[test]
fn add_recent_database_dedup_and_limit() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    cleanup_settings_file(&app);
    let service = SettingsService::new(app.handle()).expect("create service");

    service
        .add_recent_database("db-1.kdbx", None)
        .expect("add recent database");
    service
        .add_recent_database("db-2.kdbx", Some("key-2.key"))
        .expect("add recent database");
    service
        .add_recent_database("db-1.kdbx", Some("key-1.key"))
        .expect("add recent database");

    let settings = service.get_settings().expect("get settings");
    assert_eq!(settings.recent_databases.len(), 2);
    assert_eq!(settings.recent_databases[0].path, "db-1.kdbx");
    assert_eq!(
        settings.recent_databases[0].keyfile_path.as_deref(),
        Some("key-1.key")
    );

    for index in 0..(MAX_RECENT_DATABASES + 2) {
        let db_name = format!("db-{}.kdbx", index + 3);
        service
            .add_recent_database(&db_name, None)
            .expect("add recent database");
    }

    let settings = service.get_settings().expect("get settings");
    assert_eq!(settings.recent_databases.len(), MAX_RECENT_DATABASES);
    assert_eq!(settings.recent_databases[0].path, "db-14.kdbx");

    cleanup_settings_file(&app);
}

#[test]
fn keyfile_lookup_remove_and_clear() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    cleanup_settings_file(&app);
    let service = SettingsService::new(app.handle()).expect("create service");

    service
        .add_recent_database("db-1.kdbx", Some("key-1.key"))
        .expect("add recent database");

    let keyfile = service
        .get_keyfile_for_database("db-1.kdbx")
        .expect("get keyfile");
    assert_eq!(keyfile.as_deref(), Some("key-1.key"));

    service
        .remove_recent_database("db-1.kdbx")
        .expect("remove recent database");
    let keyfile = service
        .get_keyfile_for_database("db-1.kdbx")
        .expect("get keyfile");
    assert!(keyfile.is_none());

    service
        .add_recent_database("db-2.kdbx", None)
        .expect("add recent database");
    service
        .add_recent_database("db-3.kdbx", None)
        .expect("add recent database");
    service
        .clear_recent_databases()
        .expect("clear recent databases");

    let settings = service.get_settings().expect("get settings");
    assert!(settings.recent_databases.is_empty());

    cleanup_settings_file(&app);
}
