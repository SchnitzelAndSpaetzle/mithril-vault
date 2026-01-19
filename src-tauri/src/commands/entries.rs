// SPDX-License-Identifier: MIT

use crate::models::entry::{CreateEntryData, Entry, UpdateEntryData};
use crate::models::error::AppError;
use crate::services::kdbx::KdbxService;
use std::collections::BTreeMap;
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
pub async fn create_entry(
    group_id: String,
    title: String,
    username: String,
    password: String,
    url: Option<String>,
    notes: Option<String>,
    icon_id: Option<u32>,
    tags: Option<Vec<String>>,
    custom_fields: Option<BTreeMap<String, String>>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    let data = CreateEntryData {
        title,
        username,
        password,
        url,
        notes,
        icon_id,
        tags,
        custom_fields,
    };
    state.create_entry(&group_id, data)
}

#[tauri::command]
pub async fn update_entry(
    id: String,
    title: Option<String>,
    username: Option<String>,
    password: Option<String>,
    url: Option<String>,
    notes: Option<String>,
    icon_id: Option<u32>,
    tags: Option<Vec<String>>,
    custom_fields: Option<BTreeMap<String, String>>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    let data = UpdateEntryData {
        title,
        username,
        password,
        url,
        notes,
        icon_id,
        tags,
        custom_fields,
    };
    state.update_entry(&id, data)
}

#[tauri::command]
pub async fn delete_entry(
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
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
