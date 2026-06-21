// SPDX-License-Identifier: MIT

pub mod commands;
pub mod domain;
pub mod dto;
pub mod services;
pub mod utils;

use crate::dto::error::AppError;
use commands::{
    add_recent_database, clear_audit_log, clear_clipboard, clear_entry_custom_icon,
    clear_recent_databases, clear_session_key, close_database, commit_prepared_attachments,
    copy_password_to_clipboard, copy_protected_field_to_clipboard, copy_text_to_clipboard,
    create_database, create_entry, create_group, create_manual_backup, delete_backup, delete_entry,
    delete_entry_attachment, delete_group, delete_tag, export_entry_attachment,
    fetch_entry_favicon, generate_keyfile, generate_passphrase, generate_password,
    get_app_preferences, get_audit_events, get_audit_status, get_custom_icons, get_database_config,
    get_database_info, get_entry, get_entry_attachment, get_entry_password,
    get_entry_protected_custom_field, get_group, get_group_entry_counts,
    get_history_entry_password, get_history_protected_field, get_keyfile_for_database,
    get_password_health_report, get_recent_databases, get_recycle_bin_id,
    get_vault_history_settings, get_window_content_protection_supported, has_session_key,
    inspect_database, list_backups, list_entries, list_entry_history, list_groups,
    list_open_databases, lock_database, move_entry, move_group, open_database,
    open_database_with_keyfile, open_database_with_keyfile_only, prepare_dropped_attachments,
    prepare_picked_attachments, remove_recent_database, rename_group, rename_tag, report_activity,
    reset_app_preferences, restore_backup, restore_entry_history, save_database,
    set_entry_custom_icon, set_window_content_protected, store_session_key, unlock_database,
    update_app_preferences, update_entry, update_group, update_vault_history_settings,
};
use services::audit::format::Reason;
use services::audit::key::FileBackedAuditKey;
use services::audit::AuditService;
use services::auto_lock::AutoLockService;
use services::clipboard::ClipboardService;
use services::drag_drop::PendingAttachmentPaths;
use services::kdbx::KdbxService;
use services::password_health::service::PasswordHealthService;
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
        // Capture dropped file paths from the native window event *in Rust* so
        // the renderer never names a file for the add (#286, ADR-0004). The
        // paths are buffered here; the renderer inspects their sizes via
        // `prepare_dropped_attachments` (a peek) and, after resolving any
        // soft-warning confirmation, commits them via
        // `commit_prepared_attachments`, which drains the buffer.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                if let Some(buffer) = window.try_state::<Arc<PendingAttachmentPaths>>() {
                    buffer.replace(paths.clone());
                }
            }
        })
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
            get_vault_history_settings,
            update_vault_history_settings,
            get_custom_icons,
            list_open_databases,
            list_backups,
            delete_backup,
            create_manual_backup,
            restore_backup,
            generate_keyfile,
            list_entries,
            list_entry_history,
            get_entry,
            get_entry_attachment,
            prepare_picked_attachments,
            prepare_dropped_attachments,
            commit_prepared_attachments,
            export_entry_attachment,
            delete_entry_attachment,
            get_entry_password,
            get_entry_protected_custom_field,
            get_history_entry_password,
            get_history_protected_field,
            restore_entry_history,
            create_entry,
            update_entry,
            delete_entry,
            move_entry,
            fetch_entry_favicon,
            clear_entry_custom_icon,
            set_entry_custom_icon,
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
            get_audit_events,
            get_audit_status,
            clear_audit_log,
            get_app_preferences,
            update_app_preferences,
            reset_app_preferences,
            get_recent_databases,
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
            get_password_health_report,
        ])
}

fn apply_initial_window_protection<R: Runtime>(app: &AppHandle<R>) {
    let enabled = match app.try_state::<Arc<SettingsService>>() {
        Some(service) => service
            .get_settings()
            .map_or(true, |s| s.preferences.security.prevent_screen_capture),
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
    let kdbx_arc = Arc::new(kdbx_service);
    app.manage(Arc::clone(&kdbx_arc));

    // Holds the paths of an in-flight attachment-add gesture (a native
    // drag-drop or a file-picker pick) so the trusted add path can read them
    // without the renderer naming a file (#286, ADR-0004). Filled by the
    // `on_window_event` handler or `prepare_picked_attachments`, peeked by the
    // prepare step, drained by `commit_prepared_attachments`.
    app.manage(Arc::new(PendingAttachmentPaths::default()));

    let clipboard_service = ClipboardService::new();
    app.manage(Arc::new(clipboard_service));

    let settings_service = SettingsService::new(app)?;
    // Push the persisted backup config into the KDBX service so the save
    // hook honours it from first save onward. Capture the audit gate too
    // so we can apply it to AuditService below.
    let initial_audit = settings_service
        .get_app_preferences()
        .map_or((true, 90), |prefs| {
            let _ = kdbx_arc.set_backup_settings(prefs.backups);
            (prefs.audit.enabled, prefs.audit.retention_days)
        });
    app.manage(Arc::new(settings_service));

    let auto_lock_service = AutoLockService::new();
    app.manage(Arc::new(auto_lock_service));

    let data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| AppError::SecureStorage(e.to_string()))?;
    let audit_root = data_dir.join("audit");
    let key_source = Arc::new(FileBackedAuditKey::new(audit_root.join("key.bin")));
    let audit_service = AuditService::new(audit_root, key_source);
    audit_service.set_enabled(initial_audit.0);
    audit_service.set_retention_days(initial_audit.1);
    app.manage(Arc::new(audit_service));

    app.manage(Arc::new(PasswordHealthService::new()));

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
            record_app_quit_audit_events(app);
        }
        tauri::RunEvent::Resumed => {
            if let Some(kdbx) = app.try_state::<Arc<KdbxService>>() {
                if let Ok(locked_paths) = kdbx.lock_all() {
                    if !locked_paths.is_empty() {
                        if let Some(audit) = app.try_state::<Arc<AuditService>>() {
                            audit.record_vault_locked_batch(&locked_paths, Reason::ScreenLock);
                        }
                        if let Some(health) = app.try_state::<Arc<PasswordHealthService>>() {
                            for path in &locked_paths {
                                health.on_lock(path);
                            }
                        }
                        let _ = app.emit("database-locked", &locked_paths);
                    }
                }
            }
        }
        _ => {}
    });
}

/// Records one `vault.locked { reason: app_quit }` per Vault that is open
/// and unlocked at quit time. Best-effort: audit emit failures are
/// swallowed internally by `AuditService`, missing state simply means no
/// records are produced (we're exiting anyway).
fn record_app_quit_audit_events<R: Runtime>(app: &AppHandle<R>) {
    let Some(audit) = app.try_state::<Arc<AuditService>>() else {
        return;
    };
    let Some(kdbx) = app.try_state::<Arc<KdbxService>>() else {
        return;
    };
    let Ok(open) = kdbx.list_open_databases() else {
        return;
    };
    let unlocked_paths: Vec<String> = open
        .into_iter()
        .filter(|db| !db.is_locked)
        .map(|db| db.path)
        .collect();
    audit.record_vault_locked_batch(&unlocked_paths, Reason::AppQuit);
}
