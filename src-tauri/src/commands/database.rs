// SPDX-License-Identifier: MIT

use crate::dto::database::{
    DatabaseConfigDto, DatabaseCreationOptions, DatabaseHeaderInfo, DatabaseInfo,
};
use crate::dto::error::AppError;
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

/// Opens a database with a password.
#[tauri::command]
pub async fn open_database(
    path: String,
    password: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    state.open(&path, &password)
}

/// Closes the currently open database.
#[tauri::command]
pub async fn close_database(state: State<'_, Arc<KdbxService>>) -> Result<(), AppError> {
    state.close()
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

/// Saves the open database.
#[tauri::command]
pub async fn save_database(state: State<'_, Arc<KdbxService>>) -> Result<(), AppError> {
    state.save()
}

/// Opens a database with password and keyfile.
#[tauri::command]
pub async fn open_database_with_keyfile(
    path: String,
    password: String,
    keyfile_path: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    state.open_with_keyfile(&path, &password, &keyfile_path)
}

/// Opens a database using only a keyfile.
#[tauri::command]
pub async fn open_database_with_keyfile_only(
    path: String,
    keyfile_path: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    state.open_with_keyfile_only(&path, &keyfile_path)
}

/// TODO: Locks the database (not yet implemented).
#[tauri::command]
pub async fn lock_database() -> Result<(), AppError> {
    Err(AppError::NotImplemented("lock_database".into()))
}

/// TODO: Unlocks the database (not yet implemented).
#[tauri::command]
pub async fn unlock_database(password: String) -> Result<(), AppError> {
    let _ = password;
    Err(AppError::NotImplemented("unlock_database".into()))
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

/// Returns the cryptographic configuration of the currently open database.
/// Requires the database to be open (authenticated).
#[tauri::command]
pub async fn get_database_config(
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseConfigDto, AppError> {
    state.get_config()
}
