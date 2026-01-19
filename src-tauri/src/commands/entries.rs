use crate::dto::entry::{CreateEntryData, CustomFieldValue, Entry, UpdateEntryData};
use crate::dto::error::AppError;
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_entries(
    group_id: Option<String>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<Entry>, AppError> {
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
pub async fn get_entry_protected_custom_field(
    id: String,
    key: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<CustomFieldValue, AppError> {
    state.get_entry_protected_custom_field(&id, &key)
}

#[tauri::command]
pub async fn create_entry(
    group_id: String,
    data: CreateEntryData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.create_entry(&group_id, data)
}

#[tauri::command]
pub async fn update_entry(
    id: String,
    data: UpdateEntryData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.update_entry(&id, data)
}

#[tauri::command]
pub async fn delete_entry(id: String, state: State<'_, Arc<KdbxService>>) -> Result<(), AppError> {
    state.delete_entry(&id)
}

#[tauri::command]
pub async fn move_entry(
    id: String,
    target_group_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.move_entry(&id, &target_group_id)
}
