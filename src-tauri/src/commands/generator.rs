// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use crate::services::generator;

pub use crate::services::generator::{
    GeneratedPassphrase, GeneratedPassword, PassphraseGeneratorOptions, PasswordGeneratorOptions,
};

#[tauri::command]
pub async fn generate_password(
    options: PasswordGeneratorOptions,
) -> Result<GeneratedPassword, AppError> {
    generator::generate_password(&options)
}

#[tauri::command]
pub async fn generate_passphrase(
    options: PassphraseGeneratorOptions,
) -> Result<GeneratedPassphrase, AppError> {
    generator::generate_passphrase(&options)
}
