// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::error::AppError;
use crate::services::keychain::SecureStorageService;
use std::sync::Arc;
use std::time::Duration;
use tauri::State;

#[tauri::command]
pub async fn store_session_key(
    key: Vec<u8>,
    ttl_secs: Option<u64>,
    state: State<'_, Arc<SecureStorageService>>,
) -> Result<(), AppError> {
    let ttl = ttl_secs.map_or_else(
        SecureStorageService::default_session_ttl,
        Duration::from_secs,
    );
    state.store_session_key(&key, ttl)
}

#[tauri::command]
pub async fn has_session_key(
    state: State<'_, Arc<SecureStorageService>>,
) -> Result<bool, AppError> {
    state.session_key_present()
}

#[tauri::command]
pub async fn load_session_key(
    state: State<'_, Arc<SecureStorageService>>,
) -> Result<Option<Vec<u8>>, AppError> {
    state.load_session_key()
}

#[tauri::command]
pub async fn clear_session_key(
    state: State<'_, Arc<SecureStorageService>>,
) -> Result<(), AppError> {
    state.clear_session_key()
}
