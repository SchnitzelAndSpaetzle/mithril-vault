// SPDX-License-Identifier: MIT
//! Smoke tests for command handlers

#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::database::{
    get_database_info, lock_database, open_database, unlock_database,
};
use mithril_vault_lib::commands::entries::list_entries;
use mithril_vault_lib::commands::generator::{
    calculate_password_strength, generate_passphrase, generate_password,
    PassphraseGeneratorOptions, PasswordGeneratorOptions,
};
use mithril_vault_lib::commands::groups::list_groups;
use mithril_vault_lib::commands::secure_storage::{
    clear_session_key, has_session_key, store_session_key,
};
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::register_services;
use tauri::test::mock_app;
use tauri::Manager;

fn setup_app() -> tauri::App<tauri::test::MockRuntime> {
    let app = mock_app();
    register_services(app.handle()).expect("register services");
    app
}

fn cleanup_app_files(app: &tauri::App<tauri::test::MockRuntime>) {
    if let Ok(data_dir) = app.path().app_local_data_dir() {
        let _ = std::fs::remove_file(data_dir.join("settings.json"));
        let _ = std::fs::remove_file(data_dir.join("session.hold"));
    }
}

#[test]
fn generator_commands_return_not_implemented() {
    let password_err =
        tauri::async_runtime::block_on(generate_password(PasswordGeneratorOptions::default()))
            .expect_err("expected not implemented");
    assert!(matches!(password_err, AppError::NotImplemented(_)));

    let passphrase_err =
        tauri::async_runtime::block_on(generate_passphrase(PassphraseGeneratorOptions::default()))
            .expect_err("expected not implemented");
    assert!(matches!(passphrase_err, AppError::NotImplemented(_)));

    let strength_err =
        tauri::async_runtime::block_on(calculate_password_strength("test".to_string()))
            .expect_err("expected not implemented");
    assert!(matches!(strength_err, AppError::NotImplemented(_)));
}

#[test]
fn secure_storage_commands_roundtrip() {
    let app = setup_app();

    tauri::async_runtime::block_on(store_session_key(
        b"session-key".to_vec(),
        Some(3600),
        app.state(),
    ))
    .expect("store session key");

    let has_key =
        tauri::async_runtime::block_on(has_session_key(app.state())).expect("check session key");
    assert!(has_key);

    tauri::async_runtime::block_on(clear_session_key(app.state())).expect("clear session key");

    let has_key =
        tauri::async_runtime::block_on(has_session_key(app.state())).expect("check session key");
    assert!(!has_key);

    cleanup_app_files(&app);
}

#[test]
fn database_commands_handle_missing_database() {
    let app = setup_app();

    let err = tauri::async_runtime::block_on(open_database(
        "missing.kdbx".into(),
        "password".into(),
        app.state(),
    ))
    .expect_err("expected invalid path");
    assert!(matches!(err, AppError::InvalidPath(_)));

    let info =
        tauri::async_runtime::block_on(get_database_info(app.state())).expect("get database info");
    assert!(info.is_none());

    let err =
        tauri::async_runtime::block_on(lock_database()).expect_err("expected not implemented");
    assert!(matches!(err, AppError::NotImplemented(_)));

    let err = tauri::async_runtime::block_on(unlock_database("password".into()))
        .expect_err("expected not implemented");
    assert!(matches!(err, AppError::NotImplemented(_)));

    cleanup_app_files(&app);
}

#[test]
fn entries_and_groups_commands_fail_when_not_open() {
    let app = setup_app();

    let entries_err = tauri::async_runtime::block_on(list_entries(None, app.state()))
        .expect_err("expected database not open");
    assert!(matches!(entries_err, AppError::DatabaseNotOpen));

    let groups_err = tauri::async_runtime::block_on(list_groups(app.state()))
        .expect_err("expected database not open");
    assert!(matches!(groups_err, AppError::DatabaseNotOpen));

    cleanup_app_files(&app);
}
