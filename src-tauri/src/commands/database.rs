// SPDX-License-Identifier: MIT

use crate::models::database::DatabaseInfo;
use crate::models::error::AppError;
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn open_database(
    path: String,
    password: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    state.open(&path, &password)
}

#[tauri::command]
pub async fn close_database(state: State<'_, Arc<KdbxService>>) -> Result<(), AppError> {
    state.close()
}

#[tauri::command]
pub async fn create_database(
    path: String,
    password: String,
    name: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    state.create(&path, &password, &name)
}

#[tauri::command]
pub async fn save_database(state: State<'_, Arc<KdbxService>>) -> Result<(), AppError> {
    state.save()
}

#[tauri::command]
pub async fn lock_database() -> Result<(), AppError> {
    Err(AppError::NotImplemented("lock_database".into()))
}

#[tauri::command]
pub async fn unlock_database(password: String) -> Result<(), AppError> {
    let _ = password;
    Err(AppError::NotImplemented("unlock_database".into()))
}
