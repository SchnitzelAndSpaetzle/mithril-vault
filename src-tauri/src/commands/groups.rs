// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::dto::group::{Group, UpdateGroupData};
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

/// Lists groups in the database.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn list_groups(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<Group>, AppError> {
    state.list_groups(&db_id)
}

/// Fetches a group by ID.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn get_group(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Group, AppError> {
    state.get_group(&db_id, &id)
}

/// Creates a new group.
/// The `db_id` is the path to the database file.
/// `parent_id` is the parent group ID (uses root if None).
/// Frontend sends `parentId` which Tauri converts to `parent_id`.
#[tauri::command]
pub async fn create_group(
    db_id: String,
    parent_id: String,
    name: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Group, AppError> {
    state.create_group(&db_id, Some(&parent_id), &name, None)
}

/// Updates a group.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn update_group(
    db_id: String,
    id: String,
    data: UpdateGroupData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Group, AppError> {
    state.update_group(&db_id, &id, data)
}

/// Deletes a group (moves to recycle bin).
/// The `db_id` is the path to the database file.
/// Frontend sends just `id`, so `recursive` defaults to false.
#[tauri::command]
pub async fn delete_group(
    db_id: String,
    id: String,
    recursive: Option<bool>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.delete_group(&db_id, &id, recursive.unwrap_or(false), false)
}

/// Moves a group to a new parent.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn move_group(
    db_id: String,
    id: String,
    target_parent_id: Option<String>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Group, AppError> {
    state.move_group(&db_id, &id, target_parent_id.as_deref())
}

/// Renames a group (convenience wrapper around `update_group`).
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn rename_group(
    db_id: String,
    id: String,
    name: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Group, AppError> {
    state.update_group(
        &db_id,
        &id,
        UpdateGroupData {
            name: Some(name),
            icon: None,
        },
    )
}

/// Returns entry counts per group.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn get_group_entry_counts(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<std::collections::HashMap<String, u32>, AppError> {
    state.get_group_entry_counts(&db_id)
}

/// Returns the recycle bin group ID if it exists.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn get_recycle_bin_id(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Option<String>, AppError> {
    state.get_recycle_bin_id(&db_id)
}
