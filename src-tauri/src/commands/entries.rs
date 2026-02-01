use crate::dto::entry::{CreateEntryData, CustomFieldValue, Entry, UpdateEntryData};
use crate::dto::error::AppError;
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

/// Lists entries, optionally filtered by group.
/// The db_id is the path to the database file.
#[tauri::command]
pub async fn list_entries(
    db_id: String,
    group_id: Option<String>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<Entry>, AppError> {
    state.list_entries(&db_id, group_id.as_deref())
}

/// Fetches an entry by ID.
/// The db_id is the path to the database file.
#[tauri::command]
pub async fn get_entry(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.get_entry(&db_id, &id)
}

/// Fetches an entry password.
/// The db_id is the path to the database file.
#[tauri::command]
pub async fn get_entry_password(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<String, AppError> {
    state.get_entry_password(&db_id, &id)
}

/// Fetches a protected custom field value.
/// The db_id is the path to the database file.
#[tauri::command]
pub async fn get_entry_protected_custom_field(
    db_id: String,
    id: String,
    key: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<CustomFieldValue, AppError> {
    state.get_entry_protected_custom_field(&db_id, &id, &key)
}

/// Creates a new entry in a group.
/// The db_id is the path to the database file.
#[tauri::command]
pub async fn create_entry(
    db_id: String,
    group_id: String,
    data: CreateEntryData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.create_entry(&db_id, &group_id, data)
}

/// Updates an existing entry.
/// The db_id is the path to the database file.
#[tauri::command]
pub async fn update_entry(
    db_id: String,
    id: String,
    data: UpdateEntryData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.update_entry(&db_id, &id, data)
}

/// Deletes an entry by ID.
/// The db_id is the path to the database file.
#[tauri::command]
pub async fn delete_entry(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.delete_entry(&db_id, &id)
}

/// Moves an entry to another group.
/// The db_id is the path to the database file.
#[tauri::command]
pub async fn move_entry(
    db_id: String,
    id: String,
    target_group_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.move_entry(&db_id, &id, &target_group_id)
}
