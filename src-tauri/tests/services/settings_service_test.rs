// SPDX-License-Identifier: MIT
//! Tests for settings service behavior

#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::settings::AppSettings;
use mithril_vault_lib::services::settings::SettingsService;
use tauri::test::mock_app;
use tauri::Manager;

const MAX_RECENT_DATABASES: usize = 10;

fn setup_app() -> tauri::App<tauri::test::MockRuntime> {
    mock_app()
}

fn cleanup_settings_file(app: &tauri::App<tauri::test::MockRuntime>) {
    if let Ok(data_dir) = app.path().app_local_data_dir() {
        let settings_path = data_dir.join("settings.json");
        if settings_path.exists() {
            let _ = std::fs::remove_file(settings_path);
        }
    }
}

fn new_service(app: &tauri::App<tauri::test::MockRuntime>) -> SettingsService {
    SettingsService::new(app.handle()).expect("create settings service")
}

#[test]
fn default_settings_when_missing() {
    let app = setup_app();
    cleanup_settings_file(&app);

    let service = new_service(&app);
    let settings = service.get_settings().expect("get settings");

    assert_eq!(settings.auto_lock_timeout, 300);
    assert_eq!(settings.clipboard_clear_timeout, 30);
    assert_eq!(settings.theme, "system");
    assert!(settings.recent_databases.is_empty());

    cleanup_settings_file(&app);
}

#[test]
fn update_persists_across_reload() {
    let app = setup_app();
    cleanup_settings_file(&app);

    let service = new_service(&app);
    let mut updated = AppSettings::default();
    updated.auto_lock_timeout = 45;
    updated.theme = "light".into();

    service
        .update_settings(updated.clone())
        .expect("update settings");

    let reloaded = new_service(&app);
    let settings = reloaded.get_settings().expect("get settings");
    assert_eq!(settings.auto_lock_timeout, 45);
    assert_eq!(settings.theme, "light");

    cleanup_settings_file(&app);
}

#[test]
fn add_recent_database_dedup_and_limit() {
    let app = setup_app();
    cleanup_settings_file(&app);

    let service = new_service(&app);

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
    let expected_latest = format!("db-{}.kdbx", MAX_RECENT_DATABASES + 4);
    assert_eq!(settings.recent_databases.len(), MAX_RECENT_DATABASES);
    assert_eq!(settings.recent_databases[0].path, expected_latest);

    cleanup_settings_file(&app);
}

#[test]
fn keyfile_lookup_remove_and_clear() {
    let app = setup_app();
    cleanup_settings_file(&app);

    let service = new_service(&app);

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
