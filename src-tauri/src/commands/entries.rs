use crate::dto::entry::{CreateEntryData, CustomFieldValue, Entry, UpdateEntryData};
use crate::dto::error::AppError;
use crate::services::kdbx::favicons::FaviconFetchOutcome;
use crate::services::kdbx::KdbxService;
use crate::services::settings::SettingsService;
use std::sync::Arc;
use tauri::State;

/// Lists entries, optionally filtered by group.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn list_entries(
    db_id: String,
    group_id: Option<String>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<Entry>, AppError> {
    state.list_entries(&db_id, group_id.as_deref())
}

/// Fetches an entry by ID.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn get_entry(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.get_entry(&db_id, &id)
}

/// Fetches an entry password.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn get_entry_password(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<String, AppError> {
    state.get_entry_password(&db_id, &id)
}

/// Fetches a protected custom field value.
/// The `db_id` is the path to the database file.
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
/// The `db_id` is the path to the database file.
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
/// The `db_id` is the path to the database file.
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
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn delete_entry(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.delete_entry(&db_id, &id)
}

/// Renames a tag across all entries in the database.
/// Returns the number of entries that were modified.
#[tauri::command]
pub async fn rename_tag(
    db_id: String,
    old_name: String,
    new_name: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<u32, AppError> {
    state.rename_tag(&db_id, &old_name, &new_name)
}

/// Deletes a tag from all entries in the database.
/// Returns the number of entries that were modified.
#[tauri::command]
pub async fn delete_tag(
    db_id: String,
    tag_name: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<u32, AppError> {
    state.delete_tag(&db_id, &tag_name)
}

/// Moves an entry to another group.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn move_entry(
    db_id: String,
    id: String,
    target_group_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.move_entry(&db_id, &id, &target_group_id)
}

/// Fetches and stores a favicon for an entry URL as a database custom icon.
#[tauri::command]
pub async fn fetch_entry_favicon(
    db_id: String,
    id: String,
    force: Option<bool>,
    kdbx_state: State<'_, Arc<KdbxService>>,
    settings_state: State<'_, Arc<SettingsService>>,
) -> Result<FaviconFetchOutcome, AppError> {
    let settings = settings_state.get_settings()?;
    kdbx_state
        .fetch_entry_favicon(
            &db_id,
            &id,
            settings
                .preferences
                .security
                .allow_third_party_favicon_fallbacks,
            force.unwrap_or(false),
        )
        .await
}

/// Removes a custom icon assignment from an entry.
#[tauri::command]
pub async fn clear_entry_custom_icon(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<bool, AppError> {
    state.clear_entry_custom_icon(&db_id, &id)
}

/// Assigns an existing custom icon (already in the database) to an entry.
#[tauri::command]
pub async fn set_entry_custom_icon(
    db_id: String,
    id: String,
    icon_uuid: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<bool, AppError> {
    state.set_entry_custom_icon(&db_id, &id, &icon_uuid)
}
