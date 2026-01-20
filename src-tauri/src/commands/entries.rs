use crate::dto::entry::{CreateEntryData, CustomFieldValue, Entry, UpdateEntryData};
use crate::dto::error::AppError;
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

/// Lists entries, optionally filtered by group.
#[tauri::command]
pub async fn list_entries(
    group_id: Option<String>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<Entry>, AppError> {
    state.list_entries(group_id.as_deref())
}

/// Fetches an entry by ID.
#[tauri::command]
pub async fn get_entry(id: String, state: State<'_, Arc<KdbxService>>) -> Result<Entry, AppError> {
    state.get_entry(&id)
}

/// Fetches an entry password.
#[tauri::command]
pub async fn get_entry_password(
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<String, AppError> {
    state.get_entry_password(&id)
}

/// Fetches a protected custom field value.
#[tauri::command]
pub async fn get_entry_protected_custom_field(
    id: String,
    key: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<CustomFieldValue, AppError> {
    state.get_entry_protected_custom_field(&id, &key)
}

/// Creates a new entry in a group.
#[tauri::command]
pub async fn create_entry(
    group_id: String,
    data: CreateEntryData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.create_entry(&group_id, data)
}

/// Updates an existing entry.
#[tauri::command]
pub async fn update_entry(
    id: String,
    data: UpdateEntryData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.update_entry(&id, data)
}

/// Deletes an entry by ID.
#[tauri::command]
pub async fn delete_entry(id: String, state: State<'_, Arc<KdbxService>>) -> Result<(), AppError> {
    state.delete_entry(&id)
}

/// Moves an entry to another group.
#[tauri::command]
pub async fn move_entry(
    id: String,
    target_group_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.move_entry(&id, &target_group_id)
}
