// SPDX-License-Identifier: MIT

pub mod commands;
pub mod domain;
pub mod dto;
pub mod services;
pub mod utils;

use crate::dto::error::AppError;
use commands::{
    add_recent_database, clear_clipboard, clear_recent_databases, clear_session_key,
    close_database, copy_password_to_clipboard, copy_protected_field_to_clipboard,
    copy_text_to_clipboard, create_database, create_entry, create_group, delete_entry,
    delete_group, delete_tag, generate_keyfile, generate_passphrase, generate_password,
    get_app_preferences, get_custom_icons, get_database_config, get_database_info, get_entry,
    get_entry_password, get_entry_protected_custom_field, get_group, get_group_entry_counts,
    get_keyfile_for_database, get_recycle_bin_id, get_settings,
    get_window_content_protection_supported, has_session_key, inspect_database, list_entries,
    list_groups, list_open_databases, lock_database, move_entry, move_group, open_database,
    open_database_with_keyfile, open_database_with_keyfile_only, remove_recent_database,
    rename_group, rename_tag, report_activity, reset_app_preferences, save_database,
    set_window_content_protected, store_session_key, unlock_database, update_app_preferences,
    update_entry, update_group, update_settings,
};
use services::auto_lock::AutoLockService;
use services::clipboard::ClipboardService;
use services::kdbx::KdbxService;
use services::secure_storage::SecureStorageService;
use services::settings::SettingsService;
use services::window_protection::WindowProtectionService;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[doc(hidden)]
pub fn build_app<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle();
            register_services(handle)?;
            apply_initial_window_protection(handle);
            services::auto_lock::start_auto_lock_task(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_database,
            open_database_with_keyfile,
            open_database_with_keyfile_only,
            close_database,
            create_database,
            save_database,
            lock_database,
            unlock_database,
            inspect_database,
            get_database_config,
            get_database_info,
            get_custom_icons,
            list_open_databases,
            generate_keyfile,
            list_entries,
            get_entry,
            get_entry_password,
            get_entry_protected_custom_field,
            create_entry,
            update_entry,
            delete_entry,
            move_entry,
            rename_tag,
            delete_tag,
            list_groups,
            get_group,
            create_group,
            update_group,
            delete_group,
            move_group,
            rename_group,
            get_group_entry_counts,
            get_recycle_bin_id,
            generate_password,
            generate_passphrase,
            get_settings,
            update_settings,
            get_app_preferences,
            update_app_preferences,
            reset_app_preferences,
            add_recent_database,
            remove_recent_database,
            clear_recent_databases,
            get_keyfile_for_database,
            store_session_key,
            has_session_key,
            clear_session_key,
            copy_password_to_clipboard,
            copy_protected_field_to_clipboard,
            copy_text_to_clipboard,
            clear_clipboard,
            report_activity,
            set_window_content_protected,
            get_window_content_protection_supported,
        ])
}

fn apply_initial_window_protection<R: Runtime>(app: &AppHandle<R>) {
    let enabled = match app.try_state::<Arc<SettingsService>>() {
        Some(service) => service
            .get_settings()
            .map_or(true, |s| s.prevent_screen_capture),
        None => true,
    };
    if let Err(err) = WindowProtectionService::apply_to_all(app, enabled) {
        eprintln!("warning: failed to apply initial window protection: {err}");
    }
}

#[doc(hidden)]
pub fn register_services<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<(), AppError> {
    let secure_storage = SecureStorageService::new(app)?;
    app.manage(Arc::new(secure_storage));

    let kdbx_service = KdbxService::new();
    app.manage(Arc::new(kdbx_service));

    let clipboard_service = ClipboardService::new();
    app.manage(Arc::new(clipboard_service));

    let settings_service = SettingsService::new(app)?;
    app.manage(Arc::new(settings_service));

    let auto_lock_service = AutoLockService::new();
    app.manage(Arc::new(auto_lock_service));

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
/// Runs the Tauri application.
pub fn run() {
    let app = build_app(tauri::Builder::default())
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|app, event| match event {
        tauri::RunEvent::Exit => {
            if let Some(clipboard) = app.try_state::<Arc<ClipboardService>>() {
                let _ = clipboard.clear();
            }
        }
        tauri::RunEvent::Resumed => {
            if let Some(kdbx) = app.try_state::<Arc<KdbxService>>() {
                if let Ok(locked_paths) = kdbx.lock_all() {
                    if !locked_paths.is_empty() {
                        let _ = app.emit("database-locked", &locked_paths);
                    }
                }
            }
        }
        _ => {}
    });
}
