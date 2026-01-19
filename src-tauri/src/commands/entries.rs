// SPDX-License-Identifier: MIT

use crate::dto::entry::{CreateEntryData, Entry, EntryListItem, UpdateEntryData};
use crate::dto::error::AppError;
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_entries(
    group_id: Option<String>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<EntryListItem>, AppError> {
    state.list_entries(group_id.as_deref())
}

#[tauri::command]
pub async fn get_entry(id: String, state: State<'_, Arc<KdbxService>>) -> Result<Entry, AppError> {
    state.get_entry(&id)
}

#[tauri::command]
pub async fn get_entry_password(
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<String, AppError> {
    state.get_entry_password(&id)
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
