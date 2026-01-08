// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::error::AppError;

#[tauri::command]
pub async fn list_groups() -> Result<Vec<()>, AppError> {
    Err(AppError::NotImplemented("list_groups".into()))
}

#[tauri::command]
pub async fn get_group(id: String) -> Result<(), AppError> {
    let _ = id;
    Err(AppError::NotImplemented("get_group".into()))
}

#[tauri::command]
pub async fn create_group(
    parent_id: Option<String>,
    name: String,
    icon: Option<String>,
) -> Result<(), AppError> {
    let _ = (parent_id, name, icon);
    Err(AppError::NotImplemented("create_group".into()))
}

#[tauri::command]
pub async fn update_group(
    id: String,
    name: Option<String>,
    icon: Option<String>,
) -> Result<(), AppError> {
    let _ = (id, name, icon);
    Err(AppError::NotImplemented("update_group".into()))
}

#[tauri::command]
pub async fn delete_group(id: String, recursive: bool) -> Result<(), AppError> {
    let _ = (id, recursive);
    Err(AppError::NotImplemented("delete_group".into()))
}

#[tauri::command]
pub async fn move_group(id: String, target_parent_id: Option<String>) -> Result<(), AppError> {
    let _ = (id, target_parent_id);
    Err(AppError::NotImplemented("move_group".into()))
}
