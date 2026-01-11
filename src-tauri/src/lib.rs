// SPDX-License-Identifier: MIT

pub mod commands;
pub mod models;
pub mod services;
pub mod utils;

use commands::{
    add_recent_database, calculate_password_strength, clear_recent_databases, clear_session_key,
    close_database, create_database, create_entry, create_group, delete_entry, delete_group,
    generate_passphrase, generate_password, get_entry, get_entry_password, get_group, get_settings,
    has_session_key, list_entries, list_groups, lock_database, move_entry, move_group,
    open_database, remove_recent_database, save_database, store_session_key, unlock_database,
    update_entry, update_group, update_settings,
};
use services::secure_storage::SecureStorageService;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let secure_storage = SecureStorageService::new(app.handle())?;
            app.manage(Arc::new(secure_storage));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_database,
            close_database,
            create_database,
            save_database,
            lock_database,
            unlock_database,
            list_entries,
            get_entry,
            get_entry_password,
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
            generate_password,
            generate_passphrase,
            calculate_password_strength,
            get_settings,
            update_settings,
            add_recent_database,
            remove_recent_database,
            clear_recent_databases,
            store_session_key,
            has_session_key,
            clear_session_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
