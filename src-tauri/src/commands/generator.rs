// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::error::AppError;
use serde::{Deserialize, Serialize};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordGeneratorOptions {
    pub length: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
    pub exclude_chars: Option<String>,
}

impl Default for PasswordGeneratorOptions {
    fn default() -> Self {
        Self {
            length: 20,
            uppercase: true,
            lowercase: true,
            numbers: true,
            symbols: true,
            exclude_ambiguous: false,
            exclude_chars: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassphraseGeneratorOptions {
    pub word_count: usize,
    pub separator: String,
    pub capitalize: bool,
    pub include_number: bool,
}

impl Default for PassphraseGeneratorOptions {
    fn default() -> Self {
        Self {
            word_count: 4,
            separator: "-".into(),
            capitalize: true,
            include_number: true,
        }
    }
}

#[tauri::command]
pub async fn generate_password(options: PasswordGeneratorOptions) -> Result<String, AppError> {
    let _ = options;
    Err(AppError::NotImplemented("generate_password".into()))
}

#[tauri::command]
pub async fn generate_passphrase(
    options: PassphraseGeneratorOptions,
) -> Result<String, AppError> {
    let _ = options;
    Err(AppError::NotImplemented("generate_passphrase".into()))
}

#[tauri::command]
pub async fn calculate_password_strength(password: String) -> Result<u8, AppError> {
    let _ = password;
    Err(AppError::NotImplemented(
        "calculate_password_strength".into(),
    ))
}
