// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::dto::group::{CreateGroupData, Group, UpdateGroupData};
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_groups(state: State<'_, Arc<KdbxService>>) -> Result<Vec<Group>, AppError> {
    state.list_groups()
}

#[tauri::command]
pub async fn get_group(id: String, state: State<'_, Arc<KdbxService>>) -> Result<Group, AppError> {
    state.get_group(&id)
}

#[tauri::command]
pub async fn create_group(data: CreateGroupData) -> Result<Group, AppError> {
    let _ = data;
    Err(AppError::NotImplemented("create_group".into()))
}

#[tauri::command]
pub async fn update_group(id: String, data: UpdateGroupData) -> Result<Group, AppError> {
    let _ = (id, data);
    Err(AppError::NotImplemented("update_group".into()))
}

#[tauri::command]
pub async fn delete_group(id: String, recursive: bool) -> Result<(), AppError> {
    let _ = (id, recursive);
    Err(AppError::NotImplemented("delete_group".into()))
}

#[tauri::command]
pub async fn move_group(id: String, target_parent_id: Option<String>) -> Result<Group, AppError> {
    let _ = (id, target_parent_id);
    Err(AppError::NotImplemented("move_group".into()))
}
