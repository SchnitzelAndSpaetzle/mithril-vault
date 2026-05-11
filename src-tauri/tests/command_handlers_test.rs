// SPDX-License-Identifier: MIT
//! Smoke tests for command handlers

#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::clipboard::{
    clear_clipboard, copy_password_to_clipboard, copy_protected_field_to_clipboard,
};
use mithril_vault_lib::commands::database::{
    close_database, create_database, generate_keyfile, get_custom_icons, get_database_config,
    get_database_info, inspect_database, list_open_databases, lock_database, open_database,
    open_database_with_keyfile, open_database_with_keyfile_only, save_database, unlock_database,
};
use mithril_vault_lib::commands::entries::{
    clear_entry_custom_icon, delete_tag, fetch_entry_favicon, list_entries, rename_tag,
};
use mithril_vault_lib::commands::generator::{
    generate_passphrase, generate_password, PassphraseGeneratorOptions, PasswordGeneratorOptions,
};
use mithril_vault_lib::commands::groups::{
    create_group, delete_group, get_group, get_group_entry_counts, get_recycle_bin_id, list_groups,
    move_group, rename_group, update_group,
};
use mithril_vault_lib::commands::secure_storage::{
    clear_session_key, has_session_key, store_session_key,
};
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::dto::group::UpdateGroupData;
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
fn generator_commands_produce_valid_output() {
    let result =
        tauri::async_runtime::block_on(generate_password(PasswordGeneratorOptions::default()))
            .expect("expected generated password");
    assert_eq!(
        result.password.len(),
        PasswordGeneratorOptions::default().length
    );
    assert!(result.entropy_bits > 0.0);

    let passphrase =
        tauri::async_runtime::block_on(generate_passphrase(PassphraseGeneratorOptions::default()))
            .expect("expected generated passphrase");
    assert!(!passphrase.passphrase.is_empty());
    assert!(passphrase.entropy_bits > 0.0);
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
fn clipboard_copy_command_fails_when_database_is_not_open() {
    let app = setup_app();

    let err = tauri::async_runtime::block_on(copy_password_to_clipboard(
        "nonexistent.kdbx".to_string(),
        "entry-id".to_string(),
        Some(30),
        app.state(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(err, AppError::DatabaseNotFound(_)));

    cleanup_app_files(&app);
}

#[test]
fn clear_clipboard_command_returns_expected_result_shape() {
    let app = setup_app();

    let result = tauri::async_runtime::block_on(clear_clipboard(app.state()));
    assert!(
        result.is_ok() || matches!(result, Err(AppError::Io(_))),
        "clear_clipboard should either succeed or return IO error"
    );

    cleanup_app_files(&app);
}

#[test]
fn copy_protected_field_command_fails_when_database_is_not_open() {
    let app = setup_app();

    let err = tauri::async_runtime::block_on(copy_protected_field_to_clipboard(
        "nonexistent.kdbx".to_string(),
        "entry-id".to_string(),
        "secret-field".to_string(),
        Some(30),
        app.state(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(err, AppError::DatabaseNotFound(_)));

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
        tauri::async_runtime::block_on(get_database_info("missing.kdbx".to_string(), app.state()))
            .expect("get database info");
    assert!(info.is_none());

    let err =
        tauri::async_runtime::block_on(lock_database("missing.kdbx".to_string(), app.state()))
            .expect_err("expected database not found");
    assert!(matches!(err, AppError::DatabaseNotFound(_)));

    let err = tauri::async_runtime::block_on(unlock_database(
        "missing.kdbx".to_string(),
        Some("password".into()),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(err, AppError::DatabaseNotFound(_)));

    cleanup_app_files(&app);
}

#[test]
fn entries_and_groups_commands_fail_when_not_open() {
    let app = setup_app();

    let entries_err = tauri::async_runtime::block_on(list_entries(
        "nonexistent.kdbx".to_string(),
        None,
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(entries_err, AppError::DatabaseNotFound(_)));

    let rename_tag_err = tauri::async_runtime::block_on(rename_tag(
        "nonexistent.kdbx".to_string(),
        "old".to_string(),
        "new".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(rename_tag_err, AppError::DatabaseNotFound(_)));

    let delete_tag_err = tauri::async_runtime::block_on(delete_tag(
        "nonexistent.kdbx".to_string(),
        "tag".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(delete_tag_err, AppError::DatabaseNotFound(_)));

    let fetch_favicon_err = tauri::async_runtime::block_on(fetch_entry_favicon(
        "nonexistent.kdbx".to_string(),
        "entry-id".to_string(),
        Some(true),
        app.state(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(fetch_favicon_err, AppError::DatabaseNotFound(_)));

    let clear_icon_err = tauri::async_runtime::block_on(clear_entry_custom_icon(
        "nonexistent.kdbx".to_string(),
        "entry-id".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(clear_icon_err, AppError::DatabaseNotFound(_)));

    let groups_err =
        tauri::async_runtime::block_on(list_groups("nonexistent.kdbx".to_string(), app.state()))
            .expect_err("expected database not found");
    assert!(matches!(groups_err, AppError::DatabaseNotFound(_)));

    cleanup_app_files(&app);
}

#[test]
fn additional_group_and_database_commands_fail_when_not_open() {
    let app = setup_app();

    let close_err =
        tauri::async_runtime::block_on(close_database("nonexistent.kdbx".to_string(), app.state()))
            .expect_err("expected database not found");
    assert!(matches!(close_err, AppError::DatabaseNotFound(_)));

    let save_err =
        tauri::async_runtime::block_on(save_database("nonexistent.kdbx".to_string(), app.state()))
            .expect_err("expected database not found");
    assert!(matches!(save_err, AppError::DatabaseNotFound(_)));

    let config_err = tauri::async_runtime::block_on(get_database_config(
        "nonexistent.kdbx".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(config_err, AppError::DatabaseNotFound(_)));

    let icons_err = tauri::async_runtime::block_on(get_custom_icons(
        "nonexistent.kdbx".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(icons_err, AppError::DatabaseNotFound(_)));

    let open_dbs =
        tauri::async_runtime::block_on(list_open_databases(app.state())).expect("list open dbs");
    assert!(open_dbs.is_empty(), "No database should be open");

    let get_group_err = tauri::async_runtime::block_on(get_group(
        "nonexistent.kdbx".to_string(),
        "group-id".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(get_group_err, AppError::DatabaseNotFound(_)));

    let create_group_err = tauri::async_runtime::block_on(create_group(
        "nonexistent.kdbx".to_string(),
        "parent-id".to_string(),
        "Test Group".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(create_group_err, AppError::DatabaseNotFound(_)));

    let update_group_err = tauri::async_runtime::block_on(update_group(
        "nonexistent.kdbx".to_string(),
        "group-id".to_string(),
        UpdateGroupData {
            name: Some("Renamed".to_string()),
            icon: None,
        },
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(update_group_err, AppError::DatabaseNotFound(_)));

    let delete_group_err = tauri::async_runtime::block_on(delete_group(
        "nonexistent.kdbx".to_string(),
        "group-id".to_string(),
        Some(false),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(delete_group_err, AppError::DatabaseNotFound(_)));

    let move_group_err = tauri::async_runtime::block_on(move_group(
        "nonexistent.kdbx".to_string(),
        "group-id".to_string(),
        Some("target-id".to_string()),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(move_group_err, AppError::DatabaseNotFound(_)));

    let rename_group_err = tauri::async_runtime::block_on(rename_group(
        "nonexistent.kdbx".to_string(),
        "group-id".to_string(),
        "New Name".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(rename_group_err, AppError::DatabaseNotFound(_)));

    let counts_err = tauri::async_runtime::block_on(get_group_entry_counts(
        "nonexistent.kdbx".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(counts_err, AppError::DatabaseNotFound(_)));

    let recycle_err = tauri::async_runtime::block_on(get_recycle_bin_id(
        "nonexistent.kdbx".to_string(),
        app.state(),
    ))
    .expect_err("expected database not found");
    assert!(matches!(recycle_err, AppError::DatabaseNotFound(_)));

    cleanup_app_files(&app);
}

#[test]
fn database_commands_cover_success_paths() {
    let app = setup_app();
    let temp_dir = tempfile::tempdir().expect("temp dir");

    let db_path = temp_dir.path().join("command-success.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();
    let keyfile_path = temp_dir.path().join("command-success.keyx");
    let keyfile_path_str = keyfile_path.to_string_lossy().to_string();

    tauri::async_runtime::block_on(generate_keyfile(keyfile_path_str.clone()))
        .expect("generate keyfile");
    assert!(keyfile_path.exists(), "Generated keyfile should exist");

    let create_info = tauri::async_runtime::block_on(create_database(
        db_path_str.clone(),
        "Command Success Vault".to_string(),
        Some("password".to_string()),
        Some(keyfile_path_str.clone()),
        None,
        app.state(),
    ))
    .expect("create database");
    assert_eq!(create_info.path, db_path_str);

    let inspect =
        tauri::async_runtime::block_on(inspect_database(db_path_str.clone(), app.state()))
            .expect("inspect database");
    assert!(
        inspect.version.starts_with("KDBX"),
        "Inspect should report KDBX version"
    );

    let icons = tauri::async_runtime::block_on(get_custom_icons(db_path_str.clone(), app.state()))
        .expect("get custom icons");
    assert!(
        icons.is_empty(),
        "Fresh database should have no custom icons"
    );

    let open_dbs =
        tauri::async_runtime::block_on(list_open_databases(app.state())).expect("list open dbs");
    assert_eq!(open_dbs.len(), 1, "One database should be open");

    tauri::async_runtime::block_on(close_database(db_path_str.clone(), app.state()))
        .expect("close database");

    let info_after_close =
        tauri::async_runtime::block_on(get_database_info(db_path_str.clone(), app.state()))
            .expect("database info after close");
    assert!(info_after_close.is_none(), "Database should be closed");

    tauri::async_runtime::block_on(open_database_with_keyfile(
        db_path_str.clone(),
        "password".to_string(),
        keyfile_path_str.clone(),
        app.state(),
    ))
    .expect("open database with keyfile");

    tauri::async_runtime::block_on(close_database(db_path_str.clone(), app.state()))
        .expect("close database after keyfile open");

    let key_only_path = temp_dir.path().join("command-key-only.kdbx");
    let key_only_path_str = key_only_path.to_string_lossy().to_string();

    tauri::async_runtime::block_on(create_database(
        key_only_path_str.clone(),
        "Key Only Vault".to_string(),
        None,
        Some(keyfile_path_str.clone()),
        None,
        app.state(),
    ))
    .expect("create keyfile-only database");

    tauri::async_runtime::block_on(close_database(key_only_path_str.clone(), app.state()))
        .expect("close keyfile-only database");

    tauri::async_runtime::block_on(open_database_with_keyfile_only(
        key_only_path_str.clone(),
        keyfile_path_str.clone(),
        app.state(),
    ))
    .expect("open with keyfile only");

    tauri::async_runtime::block_on(close_database(key_only_path_str.clone(), app.state()))
        .expect("final close");

    cleanup_app_files(&app);
}
