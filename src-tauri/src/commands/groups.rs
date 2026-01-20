// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::dto::group::{Group, UpdateGroupData};
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

/// Lists groups in the database.
#[tauri::command]
pub async fn list_groups(state: State<'_, Arc<KdbxService>>) -> Result<Vec<Group>, AppError> {
    state.list_groups()
}

/// Fetches a group by ID.
#[tauri::command]
pub async fn get_group(id: String, state: State<'_, Arc<KdbxService>>) -> Result<Group, AppError> {
    state.get_group(&id)
}

/// Creates a new group.
/// `parent_id` is the parent group ID (uses root if None).
/// Frontend sends `parentId` which Tauri converts to `parent_id`.
#[tauri::command]
pub async fn create_group(
    parent_id: String,
    name: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Group, AppError> {
    state.create_group(Some(&parent_id), &name, None)
}

/// Updates a group.
#[tauri::command]
pub async fn update_group(
    id: String,
    data: UpdateGroupData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Group, AppError> {
    state.update_group(&id, data)
}

/// Deletes a group (moves to recycle bin).
/// Frontend sends just `id`, so `recursive` defaults to false.
#[tauri::command]
pub async fn delete_group(
    id: String,
    recursive: Option<bool>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.delete_group(&id, recursive.unwrap_or(false), false)
}

/// Moves a group to a new parent.
#[tauri::command]
pub async fn move_group(
    id: String,
    target_parent_id: Option<String>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Group, AppError> {
    state.move_group(&id, target_parent_id.as_deref())
}

/// Renames a group (convenience wrapper around `update_group`).
#[tauri::command]
pub async fn rename_group(
    id: String,
    name: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Group, AppError> {
    state.update_group(
        &id,
        UpdateGroupData {
            name: Some(name),
            icon: None,
        },
    )
}
