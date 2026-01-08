// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::error::AppError;

#[tauri::command]
pub async fn open_database(path: String, password: String) -> Result<(), AppError> {
    let _ = (path, password);
    Err(AppError::NotImplemented("open_database".into()))
}

#[tauri::command]
pub async fn close_database() -> Result<(), AppError> {
    Err(AppError::NotImplemented("close_database".into()))
}

#[tauri::command]
pub async fn create_database(
    path: String,
    password: String,
    name: String,
) -> Result<(), AppError> {
    let _ = (path, password, name);
    Err(AppError::NotImplemented("create_database".into()))
}

#[tauri::command]
pub async fn save_database() -> Result<(), AppError> {
    Err(AppError::NotImplemented("save_database".into()))
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
