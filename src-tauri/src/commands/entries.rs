// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::error::AppError;

#[tauri::command]
pub async fn list_entries(group_id: Option<String>) -> Result<Vec<()>, AppError> {
    let _ = group_id;
    Err(AppError::NotImplemented("list_entries".into()))
}

#[tauri::command]
pub async fn get_entry(id: String) -> Result<(), AppError> {
    let _ = id;
    Err(AppError::NotImplemented("get_entry".into()))
}

#[tauri::command]
pub async fn get_entry_password(id: String) -> Result<String, AppError> {
    let _ = id;
    Err(AppError::NotImplemented("get_entry_password".into()))
}

#[tauri::command]
pub async fn create_entry(
    group_id: String,
    title: String,
    username: String,
    password: String,
    url: Option<String>,
    notes: Option<String>,
) -> Result<(), AppError> {
    let _ = (group_id, title, username, password, url, notes);
    Err(AppError::NotImplemented("create_entry".into()))
}

#[tauri::command]
pub async fn update_entry(
    id: String,
    title: Option<String>,
    username: Option<String>,
    password: Option<String>,
    url: Option<String>,
    notes: Option<String>,
) -> Result<(), AppError> {
    let _ = (id, title, username, password, url, notes);
    Err(AppError::NotImplemented("update_entry".into()))
}

#[tauri::command]
pub async fn delete_entry(id: String) -> Result<(), AppError> {
    let _ = id;
    Err(AppError::NotImplemented("delete_entry".into()))
}

#[tauri::command]
pub async fn move_entry(id: String, target_group_id: String) -> Result<(), AppError> {
    let _ = (id, target_group_id);
    Err(AppError::NotImplemented("move_entry".into()))
}
