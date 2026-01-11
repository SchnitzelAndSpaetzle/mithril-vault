// SPDX-License-Identifier: MIT

use crate::models::entry::{CreateEntryData, Entry, EntryListItem, UpdateEntryData};
use crate::models::error::AppError;

#[tauri::command]
pub async fn list_entries(group_id: Option<String>) -> Result<Vec<EntryListItem>, AppError> {
    let _ = group_id;
    Err(AppError::NotImplemented("list_entries".into()))
}

#[tauri::command]
pub async fn get_entry(id: String) -> Result<Entry, AppError> {
    let _ = id;
    Err(AppError::NotImplemented("get_entry".into()))
}

#[tauri::command]
pub async fn get_entry_password(id: String) -> Result<String, AppError> {
    let _ = id;
    Err(AppError::NotImplemented("get_entry_password".into()))
}

#[tauri::command]
pub async fn create_entry(data: CreateEntryData) -> Result<Entry, AppError> {
    let _ = data;
    Err(AppError::NotImplemented("create_entry".into()))
}

#[tauri::command]
pub async fn update_entry(id: String, data: UpdateEntryData) -> Result<Entry, AppError> {
    let _ = (id, data);
    Err(AppError::NotImplemented("update_entry".into()))
}

#[tauri::command]
pub async fn delete_entry(id: String) -> Result<(), AppError> {
    let _ = id;
    Err(AppError::NotImplemented("delete_entry".into()))
}

#[tauri::command]
pub async fn move_entry(id: String, target_group_id: String) -> Result<Entry, AppError> {
    let _ = (id, target_group_id);
    Err(AppError::NotImplemented("move_entry".into()))
}
