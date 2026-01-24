// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::services::secure_storage::SecureStorageService;
use std::sync::Arc;
use std::time::Duration;
use tauri::State;
use zeroize::Zeroize;

/// Stores a session key for the backend.
#[tauri::command]
pub async fn store_session_key(
    mut key: Vec<u8>,
    ttl_secs: Option<u64>,
    state: State<'_, Arc<SecureStorageService>>,
) -> Result<(), AppError> {
    let ttl = ttl_secs.map_or_else(
        SecureStorageService::default_session_ttl,
        Duration::from_secs,
    );
    let result = state.store_session_key(&key, ttl);
    key.zeroize();
    result
}

/// Checks whether a session key is stored.
#[tauri::command]
pub async fn has_session_key(
    state: State<'_, Arc<SecureStorageService>>,
) -> Result<bool, AppError> {
    state.session_key_present()
}

// Note: load_session_key is intentionally NOT exposed as a command.
// Session keys should remain in the Rust boundary and never be sent to the frontend.
// Use has_session_key to check presence, and unlock_database_with_session (when implemented)
// to use the stored key for database operations.

/// Clears the stored session key.
#[tauri::command]
pub async fn clear_session_key(
    state: State<'_, Arc<SecureStorageService>>,
) -> Result<(), AppError> {
    state.clear_session_key()
}
