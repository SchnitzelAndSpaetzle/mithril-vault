// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::services::window_protection::WindowProtectionService;
use tauri::{AppHandle, Runtime};

/// Toggles the OS-level capture protection on every webview window.
#[tauri::command]
pub async fn set_window_content_protected<R: Runtime>(
    enabled: bool,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    WindowProtectionService::apply_to_all(&app, enabled)
}

/// Reports whether the host OS enforces window content protection.
#[tauri::command]
pub async fn get_window_content_protection_supported() -> Result<bool, AppError> {
    Ok(WindowProtectionService::is_supported())
}
