// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
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
    generate_password_value(&options)
}

fn generate_password_value(options: &PasswordGeneratorOptions) -> Result<String, AppError> {
    if options.length == 0 {
        return Err(AppError::Crypto(
            "Password length must be greater than zero".into(),
        ));
    }

    let exclude_chars = options.exclude_chars.clone().unwrap_or_default();
    let exclude_set: std::collections::HashSet<char> = exclude_chars.chars().collect();

    let mut charset = Vec::new();

    if options.uppercase {
        charset.extend(filter_charset(
            UPPERCASE_CHARS,
            options.exclude_ambiguous,
            &exclude_set,
        ));
    }
    if options.lowercase {
        charset.extend(filter_charset(
            LOWERCASE_CHARS,
            options.exclude_ambiguous,
            &exclude_set,
        ));
    }
    if options.numbers {
        charset.extend(filter_charset(
            NUMBER_CHARS,
            options.exclude_ambiguous,
            &exclude_set,
        ));
    }
    if options.symbols {
        charset.extend(filter_charset(SYMBOL_CHARS, false, &exclude_set));
    }

    if charset.is_empty() {
        return Err(AppError::Crypto(
            "No characters available for password generation. Adjust character set options.".into(),
        ));
    }

    let mut rng = OsRng;
    let mut password = String::with_capacity(options.length);

    for _ in 0..options.length {
        let ch = charset.choose(&mut rng).ok_or_else(|| {
            AppError::Crypto("Failed to select a random character for password generation".into())
        })?;
        password.push(*ch);
    }

    Ok(password)
}

fn filter_charset(
    source: &str,
    exclude_ambiguous: bool,
    excluded_chars: &std::collections::HashSet<char>,
) -> Vec<char> {
    source
        .chars()
        .filter(|ch| {
            !excluded_chars.contains(ch) && (!exclude_ambiguous || !AMBIGUOUS_CHARS.contains(ch))
        })
        .collect()
}

const UPPERCASE_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE_CHARS: &str = "abcdefghijklmnopqrstuvwxyz";
const NUMBER_CHARS: &str = "0123456789";
const SYMBOL_CHARS: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
const AMBIGUOUS_CHARS: [char; 5] = ['0', 'O', '1', 'l', 'I'];

/// TODO: Generates a passphrase (not yet implemented).
#[tauri::command]
pub async fn generate_passphrase(options: PassphraseGeneratorOptions) -> Result<String, AppError> {
    let _ = options;
    Err(AppError::NotImplemented("generate_passphrase".into()))
}

/// TODO: Calculates password strength (not yet implemented).
#[tauri::command]
pub async fn calculate_password_strength(password: String) -> Result<u8, AppError> {
    let _ = password;
    Err(AppError::NotImplemented(
        "calculate_password_strength".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_password_with_requested_length() {
        let options = PasswordGeneratorOptions {
            length: 32,
            ..Default::default()
        };

        let password = generate_password_value(&options).expect("password generation should work");
        assert_eq!(password.len(), 32);
    }

    #[test]
    fn rejects_when_all_character_sets_disabled() {
        let options = PasswordGeneratorOptions {
            uppercase: false,
            lowercase: false,
            numbers: false,
            symbols: false,
            ..Default::default()
        };

        let result = generate_password_value(&options);
        assert!(result.is_err());
    }

    #[test]
    fn excludes_ambiguous_characters_when_requested() {
        let options = PasswordGeneratorOptions {
            length: 128,
            exclude_ambiguous: true,
            ..Default::default()
        };

        let password = generate_password_value(&options).expect("password generation should work");
        assert!(password.chars().all(|ch| !AMBIGUOUS_CHARS.contains(&ch)));
    }

    #[test]
    fn excludes_custom_characters() {
        let options = PasswordGeneratorOptions {
            length: 64,
            exclude_chars: Some("abcXYZ123".into()),
            ..Default::default()
        };

        let password = generate_password_value(&options).expect("password generation should work");
        assert!(password
            .chars()
            .all(|ch| !['a', 'b', 'c', 'X', 'Y', 'Z', '1', '2', '3'].contains(&ch)));
    }
}
