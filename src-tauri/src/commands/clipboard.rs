// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::services::clipboard::ClipboardService;
use crate::services::kdbx::KdbxService;
use std::sync::Arc;
use tauri::State;

/// Copies an entry's password to the clipboard with optional auto-clear.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn copy_password_to_clipboard(
    db_id: String,
    entry_id: String,
    timeout_secs: Option<u32>,
    kdbx: State<'_, Arc<KdbxService>>,
    clipboard: State<'_, Arc<ClipboardService>>,
) -> Result<(), AppError> {
    let password = kdbx.get_entry_password(&db_id, &entry_id)?;
    clipboard.copy(&password, timeout_secs)
}

#[tauri::command]
pub async fn copy_text_to_clipboard(
    text: String,
    timeout_secs: Option<u32>,
    clipboard: State<'_, Arc<ClipboardService>>,
) -> Result<(), AppError> {
    clipboard.copy(&text, timeout_secs)
}

/// Copies a protected custom field to the clipboard with optional auto-clear.
#[tauri::command]
pub async fn copy_protected_field_to_clipboard(
    db_id: String,
    entry_id: String,
    field_key: String,
    timeout_secs: Option<u32>,
    kdbx: State<'_, Arc<KdbxService>>,
    clipboard: State<'_, Arc<ClipboardService>>,
) -> Result<(), AppError> {
    let field = kdbx.get_entry_protected_custom_field(&db_id, &entry_id, &field_key)?;
    clipboard.copy(&field.value, timeout_secs)
}

/// Clears the clipboard.
#[tauri::command]
pub async fn clear_clipboard(clipboard: State<'_, Arc<ClipboardService>>) -> Result<(), AppError> {
    clipboard.clear()
}
