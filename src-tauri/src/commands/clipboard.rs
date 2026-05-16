// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::services::audit::AuditService;
use crate::services::clipboard::ClipboardService;
use crate::services::kdbx::KdbxService;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

/// Maps a clipboard password-copy outcome through the audit subsystem:
/// a successful copy records exactly one `entry.password_copied` event;
/// any error leaves the log untouched so the audit reflects what
/// actually landed on the user's clipboard.
///
/// Free function (mirroring `commands::entries::audit_entry_password_revealed_on_success`)
/// so integration tests can drive it without a Tauri runtime.
pub fn audit_entry_password_copied_on_success<T>(
    audit: &AuditService,
    vault_path: &str,
    entry_id: &str,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    if result.is_ok() {
        audit.record_entry_password_copied(Path::new(vault_path), entry_id);
    }
    result
}

/// Copies an entry's password to the clipboard with optional auto-clear.
/// A successful copy also appends one `entry.password_copied` event to
/// the open Vault's audit log.
#[tauri::command]
pub async fn copy_password_to_clipboard(
    db_id: String,
    entry_id: String,
    timeout_secs: Option<u32>,
    kdbx: State<'_, Arc<KdbxService>>,
    clipboard: State<'_, Arc<ClipboardService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<(), AppError> {
    let password = kdbx.get_entry_password(&db_id, &entry_id)?;
    audit_entry_password_copied_on_success(
        audit.inner(),
        &db_id,
        &entry_id,
        clipboard.copy(&password, timeout_secs),
    )
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
