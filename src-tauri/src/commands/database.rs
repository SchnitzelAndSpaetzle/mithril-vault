// SPDX-License-Identifier: MIT

use crate::dto::database::{
    CustomIconData, DatabaseConfigDto, DatabaseCreationOptions, DatabaseHeaderInfo, DatabaseInfo,
};
use crate::dto::error::AppError;
use crate::services::auto_lock::AutoLockService;
use crate::services::kdbx::backups::BackupError;
use crate::services::kdbx::KdbxService;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};

/// Payload of the `backup-warning` event. The frontend renders this as a
/// non-blocking toast and never as a modal — open-side backup failures must
/// not interrupt the user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupWarningPayload {
    path: String,
    reason: String,
}

/// Invokes the open-side backup hook and converts a `BackupFailed` into a
/// non-blocking `backup-warning` event. Never returns an error — the unlock
/// has already succeeded by the time we reach this point and a snapshot
/// problem must not bubble up to the caller.
fn emit_open_backup_hook<R: Runtime>(app: &AppHandle<R>, state: &KdbxService, db_id: &str) {
    match state.snapshot_after_open(db_id) {
        Ok(_) => {}
        Err(BackupError::BackupFailed { path, source }) => {
            let _ = app.emit(
                "backup-warning",
                BackupWarningPayload {
                    path: path.to_string_lossy().into_owned(),
                    reason: source.to_string(),
                },
            );
        }
    }
}

/// Opens a database with a password.
#[tauri::command]
pub async fn open_database<R: Runtime>(
    path: String,
    password: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    let info = state.open(&path, &password)?;
    emit_open_backup_hook(&app, &state, &path);
    Ok(info)
}

/// Closes a specific open database.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn close_database(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.close(&db_id)
}

/// Create a new KDBX4 database
///
/// # Parameters
/// - `path`: File path where the database will be saved
/// - `name`: Database name (also used as root group name)
/// - `password`: Optional password (required if no keyfile)
/// - `keyfile_path`: Optional path to keyfile for authentication
/// - `options`: Optional creation options (KDF settings, default groups, description)
#[tauri::command]
pub async fn create_database(
    path: String,
    name: String,
    password: Option<String>,
    keyfile_path: Option<String>,
    options: Option<DatabaseCreationOptions>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    state.create_database(
        &path,
        password.as_deref(),
        keyfile_path.as_deref(),
        &name,
        &options.unwrap_or_default(),
    )
}

/// Saves a specific open database.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn save_database(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.save(&db_id)
}

/// Opens a database with password and keyfile.
#[tauri::command]
pub async fn open_database_with_keyfile<R: Runtime>(
    path: String,
    password: String,
    keyfile_path: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    let info = state.open_with_keyfile(&path, &password, &keyfile_path)?;
    emit_open_backup_hook(&app, &state, &path);
    Ok(info)
}

/// Opens a database using only a keyfile.
#[tauri::command]
pub async fn open_database_with_keyfile_only<R: Runtime>(
    path: String,
    keyfile_path: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    let info = state.open_with_keyfile_only(&path, &keyfile_path)?;
    emit_open_backup_hook(&app, &state, &path);
    Ok(info)
}

/// Locks the database session by dropping decrypted data from memory.
#[tauri::command]
pub async fn lock_database(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    state.lock(&db_id)
}

/// Unlocks the database session by re-opening from disk with optional password.
#[tauri::command]
pub async fn unlock_database<R: Runtime>(
    db_id: String,
    password: Option<String>,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    // Snapshot the locked state BEFORE calling unlock so we can tell apart
    // an actual locked → unlocked transition from the no-op case where the
    // caller invoked unlock on an already-unlocked DB. Without this guard
    // the open-side backup hook would re-fire on every redundant unlock —
    // wasted work in the happy path, and a duplicated `backup-warning`
    // event whenever the backup dir is broken.
    let was_locked = state.is_database_locked(&db_id)?.unwrap_or(true);
    let info = state.unlock(&db_id, password.as_deref())?;
    if was_locked {
        emit_open_backup_hook(&app, &state, &db_id);
    }
    Ok(info)
}

/// Inspects a KDBX file without requiring credentials.
/// Returns header information including version and validity status.
#[tauri::command]
pub async fn inspect_database(
    path: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseHeaderInfo, AppError> {
    state.inspect(&path)
}

/// Returns the cryptographic configuration of a specific open database.
/// Requires the database to be open (authenticated).
#[tauri::command]
pub async fn get_database_config(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseConfigDto, AppError> {
    state.get_config(&db_id)
}

/// Gets info about a specific open database.
/// The `db_id` is the path to the database file.
/// Returns None if the database is not open.
#[tauri::command]
pub async fn get_database_info(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Option<DatabaseInfo>, AppError> {
    match state.get_info(&db_id) {
        Ok(info) => Ok(Some(info)),
        Err(AppError::DatabaseNotFound(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Returns custom icons for a specific open database.
#[tauri::command]
pub async fn get_custom_icons(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<std::collections::HashMap<String, CustomIconData>, AppError> {
    state.get_custom_icons(&db_id)
}

/// Lists all currently open databases.
#[tauri::command]
pub async fn list_open_databases(
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<DatabaseInfo>, AppError> {
    state.list_open_databases()
}

/// Generates a new `KeePass` 2.x compatible keyfile (.keyx format).
///
/// The keyfile contains 32 bytes of cryptographically random data
/// in an XML format compatible with `KeePass` 2.x and other implementations.
///
/// # Parameters
/// - `output_path`: Path where the keyfile will be saved
#[tauri::command]
pub async fn generate_keyfile(output_path: String) -> Result<(), AppError> {
    crate::services::kdbx::keyfile::generate_keyfile(&output_path)
}

/// Reports user activity to reset the auto-lock timeout.
#[tauri::command]
pub async fn report_activity(state: State<'_, Arc<AutoLockService>>) -> Result<(), AppError> {
    state.report_activity();
    Ok(())
}
