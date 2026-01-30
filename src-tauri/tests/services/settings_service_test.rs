// SPDX-License-Identifier: MIT
//! Tests for settings service behavior

#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::settings::AppSettings;
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::settings::SettingsService;
use serde_json;
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
            let _ = std::fs::remove_file(&settings_path);
            let _ = std::fs::remove_dir_all(&settings_path);
        }
        if let Ok(entries) = std::fs::read_dir(data_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                if file_name
                    .to_string_lossy()
                    .starts_with("settings.json.bad-")
                {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

fn settings_file_path(app: &tauri::App<tauri::test::MockRuntime>) -> std::path::PathBuf {
    app.path()
        .app_local_data_dir()
        .expect("app data dir")
        .join("settings.json")
}

fn create_settings_dir(app: &tauri::App<tauri::test::MockRuntime>) -> std::path::PathBuf {
    let data_dir = app.path().app_local_data_dir().expect("app data dir");
    let settings_path = data_dir.join("settings.json");
    let _ = std::fs::remove_file(&settings_path);
    let _ = std::fs::remove_dir_all(&settings_path);
    std::fs::create_dir_all(&settings_path).expect("create settings dir");
    settings_path
}

fn cleanup_settings_dir(settings_path: std::path::PathBuf) {
    let _ = std::fs::remove_dir_all(settings_path);
}

fn new_service(app: &tauri::App<tauri::test::MockRuntime>) -> SettingsService {
    SettingsService::new(app.handle()).expect("create settings service")
}

#[test]
fn default_settings_when_missing() {
    let _lock = crate::settings_test_lock();
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
    let _lock = crate::settings_test_lock();
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
fn load_settings_from_existing_file() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    cleanup_settings_file(&app);

    let settings_path = settings_file_path(&app);
    let settings = AppSettings {
        auto_lock_timeout: 120,
        clipboard_clear_timeout: 45,
        show_password_by_default: true,
        minimize_to_tray: false,
        start_minimized: true,
        theme: "dark".into(),
        recent_databases: Vec::new(),
    };
    let content = serde_json::to_string_pretty(&settings).expect("serialize settings");
    std::fs::write(&settings_path, content).expect("write settings");

    let service = new_service(&app);
    let loaded = service.get_settings().expect("get settings");
    assert_eq!(loaded.auto_lock_timeout, 120);
    assert_eq!(loaded.clipboard_clear_timeout, 45);
    assert!(loaded.show_password_by_default);
    assert!(!loaded.minimize_to_tray);
    assert!(loaded.start_minimized);
    assert_eq!(loaded.theme, "dark");

    cleanup_settings_file(&app);
}

#[test]
fn invalid_settings_falls_back_and_backs_up() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    cleanup_settings_file(&app);

    let data_dir = app.path().app_local_data_dir().expect("app data dir");
    let settings_path = data_dir.join("settings.json");
    std::fs::write(&settings_path, "{ invalid").expect("write invalid settings");

    let service = new_service(&app);
    let settings = service.get_settings().expect("get settings");

    assert_eq!(settings.auto_lock_timeout, 300);
    assert_eq!(settings.clipboard_clear_timeout, 30);
    assert_eq!(settings.theme, "system");

    let backups = std::fs::read_dir(&data_dir)
        .expect("read data dir")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("settings.json.bad-")
        })
        .count();
    assert!(backups >= 1);

    cleanup_settings_file(&app);
}

#[test]
fn add_recent_database_dedup_and_limit() {
    let _lock = crate::settings_test_lock();
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
    let _lock = crate::settings_test_lock();
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

#[test]
fn get_keyfile_returns_none_for_missing_path() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    cleanup_settings_file(&app);

    let service = new_service(&app);
    let keyfile = service
        .get_keyfile_for_database("missing.kdbx")
        .expect("get keyfile");
    assert!(keyfile.is_none());

    cleanup_settings_file(&app);
}

#[test]
fn save_error_surfaces_as_io_error() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    let service = new_service(&app);
    let settings_path = create_settings_dir(&app);
    let mut updated = AppSettings::default();
    updated.theme = "light".into();

    let err = service
        .update_settings(updated)
        .expect_err("expected io error");
    assert!(matches!(err, AppError::Io(_)));

    cleanup_settings_dir(settings_path);
}

#[test]
fn add_recent_database_surfaces_save_error() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    let service = new_service(&app);
    let settings_path = create_settings_dir(&app);
    let err = service
        .add_recent_database("db-1.kdbx", None)
        .expect_err("expected io error");
    assert!(matches!(err, AppError::Io(_)));

    cleanup_settings_dir(settings_path);
}

#[test]
fn remove_recent_database_surfaces_save_error() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    let service = new_service(&app);
    let settings_path = create_settings_dir(&app);
    let err = service
        .remove_recent_database("db-1.kdbx")
        .expect_err("expected io error");
    assert!(matches!(err, AppError::Io(_)));

    cleanup_settings_dir(settings_path);
}

#[test]
fn clear_recent_databases_surfaces_save_error() {
    let _lock = crate::settings_test_lock();
    let app = setup_app();
    let service = new_service(&app);
    let settings_path = create_settings_dir(&app);
    let err = service
        .clear_recent_databases()
        .expect_err("expected io error");
    assert!(matches!(err, AppError::Io(_)));

    cleanup_settings_dir(settings_path);
}
