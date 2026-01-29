// SPDX-License-Identifier: MIT

pub mod commands;
pub mod domain;
pub mod dto;
pub mod services;
pub mod utils;

use crate::dto::error::AppError;
use commands::{
    add_recent_database, calculate_password_strength, clear_recent_databases, clear_session_key,
    close_database, create_database, create_entry, create_group, delete_entry, delete_group,
    force_unlock_database, generate_passphrase, generate_password, get_database_config, get_entry,
    get_entry_password, get_entry_protected_custom_field, get_group, get_keyfile_for_database,
    get_lock_status, get_settings, has_session_key, inspect_database, list_entries, list_groups,
    lock_database, move_entry, move_group, open_database, open_database_with_keyfile,
    open_database_with_keyfile_only, remove_recent_database, rename_group, save_database,
    store_session_key, unlock_database, update_entry, update_group, update_settings,
};
use services::kdbx::KdbxService;
use services::secure_storage::SecureStorageService;
use services::settings::SettingsService;
use std::sync::Arc;
use tauri::{Manager, Runtime};

#[doc(hidden)]
pub fn build_app<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| register_services(app.handle()).map_err(Into::into))
        .invoke_handler(tauri::generate_handler![
            open_database,
            open_database_with_keyfile,
            open_database_with_keyfile_only,
            close_database,
            create_database,
            save_database,
            lock_database,
            unlock_database,
            get_lock_status,
            force_unlock_database,
            inspect_database,
            get_database_config,
            list_entries,
            get_entry,
            get_entry_password,
            get_entry_protected_custom_field,
            create_entry,
            update_entry,
            delete_entry,
            move_entry,
            list_groups,
            get_group,
            create_group,
            update_group,
            delete_group,
            move_group,
            rename_group,
            generate_password,
            generate_passphrase,
            calculate_password_strength,
            get_settings,
            update_settings,
            add_recent_database,
            remove_recent_database,
            clear_recent_databases,
            get_keyfile_for_database,
            store_session_key,
            has_session_key,
            clear_session_key,
        ])
}

#[doc(hidden)]
pub fn register_services<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<(), AppError> {
    let secure_storage = SecureStorageService::new(app)?;
    app.manage(Arc::new(secure_storage));

    let kdbx_service = KdbxService::new();
    app.manage(Arc::new(kdbx_service));

    let settings_service = SettingsService::new(app)?;
    app.manage(Arc::new(settings_service));

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
/// Runs the Tauri application.
pub fn run() {
    build_app(tauri::Builder::default())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
