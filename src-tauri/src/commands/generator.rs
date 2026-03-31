// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

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
    pub min_numbers: Option<usize>,
    pub min_symbols: Option<usize>,
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
            min_numbers: None,
            min_symbols: None,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedPassword {
    pub password: String,
    pub entropy_bits: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedPassphrase {
    pub passphrase: String,
    pub entropy_bits: f64,
}

#[tauri::command]
pub async fn generate_password(
    options: PasswordGeneratorOptions,
) -> Result<GeneratedPassword, AppError> {
    generate_password_value(&options)
}

fn validate_min_requirements(options: &PasswordGeneratorOptions) -> Result<(), AppError> {
    if let Some(min_numbers) = options.min_numbers {
        if !options.numbers {
            return Err(AppError::Crypto(
                "Cannot require minimum numbers when numbers are disabled".into(),
            ));
        }
        if min_numbers > options.length {
            return Err(AppError::Crypto(
                "Minimum numbers cannot exceed password length".into(),
            ));
        }
    }
    if let Some(min_symbols) = options.min_symbols {
        if !options.symbols {
            return Err(AppError::Crypto(
                "Cannot require minimum symbols when symbols are disabled".into(),
            ));
        }
        if min_symbols > options.length {
            return Err(AppError::Crypto(
                "Minimum symbols cannot exceed password length".into(),
            ));
        }
    }
    let total_min = options.min_numbers.unwrap_or(0) + options.min_symbols.unwrap_or(0);
    if total_min > options.length {
        return Err(AppError::Crypto(
            "Combined minimum requirements exceed password length".into(),
        ));
    }
    Ok(())
}

fn generate_password_value(
    options: &PasswordGeneratorOptions,
) -> Result<GeneratedPassword, AppError> {
    if options.length == 0 {
        return Err(AppError::Crypto(
            "Password length must be greater than zero".into(),
        ));
    }

    validate_min_requirements(options)?;

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

    let number_chars: Vec<char> = if options.numbers {
        let chars = filter_charset(NUMBER_CHARS, options.exclude_ambiguous, &exclude_set);
        charset.extend(&chars);
        chars
    } else {
        Vec::new()
    };

    let symbol_chars: Vec<char> = if options.symbols {
        let chars = filter_charset(SYMBOL_CHARS, false, &exclude_set);
        charset.extend(&chars);
        chars
    } else {
        Vec::new()
    };

    if charset.is_empty() {
        return Err(AppError::Crypto(
            "No characters available for password generation. Adjust character set options.".into(),
        ));
    }

    let charset_size = charset.len();
    let mut rng = OsRng;
    let mut password: Vec<char> = Vec::with_capacity(options.length);

    for _ in 0..options.length {
        let ch = charset.choose(&mut rng).ok_or_else(|| {
            AppError::Crypto("Failed to select a random character for password generation".into())
        })?;
        password.push(*ch);
    }

    // Enforce minimum character requirements by replacing random positions
    enforce_minimum(
        &mut password,
        &number_chars,
        options.min_numbers,
        &charset,
        &mut rng,
    )?;
    enforce_minimum(
        &mut password,
        &symbol_chars,
        options.min_symbols,
        &charset,
        &mut rng,
    )?;

    // Shuffle to avoid positional bias from enforcement
    password.shuffle(&mut rng);

    #[allow(clippy::cast_precision_loss)]
    let entropy_bits = options.length as f64 * (charset_size as f64).log2();

    Ok(GeneratedPassword {
        password: password.into_iter().collect(),
        entropy_bits,
    })
}

fn enforce_minimum(
    password: &mut [char],
    required_chars: &[char],
    min_count: Option<usize>,
    _full_charset: &[char],
    rng: &mut OsRng,
) -> Result<(), AppError> {
    let Some(min) = min_count else {
        return Ok(());
    };
    if min == 0 || required_chars.is_empty() {
        return Ok(());
    }

    let current_count = password
        .iter()
        .filter(|ch| required_chars.contains(ch))
        .count();

    if current_count >= min {
        return Ok(());
    }

    let needed = min - current_count;

    // Find positions that don't already contain a required character
    let mut replaceable: Vec<usize> = password
        .iter()
        .enumerate()
        .filter(|(_, ch)| !required_chars.contains(ch))
        .map(|(i, _)| i)
        .collect();

    replaceable.shuffle(rng);

    for &pos in replaceable.iter().take(needed) {
        let ch = required_chars
            .choose(rng)
            .ok_or_else(|| AppError::Crypto("Failed to select replacement character".into()))?;
        password[pos] = *ch;
    }

    Ok(())
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

// EFF large diceware wordlist (7776 words)
static WORDLIST: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    include_str!("../../assets/eff-diceware-wordlist.txt")
        .lines()
        .filter(|line| !line.is_empty())
        .collect()
});

#[tauri::command]
pub async fn generate_passphrase(
    options: PassphraseGeneratorOptions,
) -> Result<GeneratedPassphrase, AppError> {
    generate_passphrase_value(&options)
}

fn generate_passphrase_value(
    options: &PassphraseGeneratorOptions,
) -> Result<GeneratedPassphrase, AppError> {
    let wordlist = &*WORDLIST;

    if wordlist.is_empty() {
        return Err(AppError::Crypto("Wordlist is empty".into()));
    }

    if options.word_count == 0 {
        return Err(AppError::Crypto(
            "Word count must be greater than zero".into(),
        ));
    }

    let mut rng = OsRng;
    let mut words: Vec<String> = Vec::with_capacity(options.word_count);

    for _ in 0..options.word_count {
        let word = wordlist
            .choose(&mut rng)
            .ok_or_else(|| AppError::Crypto("Failed to select a random word".into()))?;

        let word = if options.capitalize {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        } else {
            (*word).to_string()
        };

        words.push(word);
    }

    // Append a random digit to a random word
    if options.include_number {
        let idx = rng.gen_range(0..words.len());
        let digit = rng.gen_range(0..10);
        words[idx].push_str(&digit.to_string());
    }

    let passphrase = words.join(&options.separator);

    // Entropy: each word is chosen from 7776 options = ~12.9 bits per word
    #[allow(clippy::cast_precision_loss)]
    let wordlist_size = wordlist.len() as f64;
    #[allow(clippy::cast_precision_loss)]
    let mut entropy_bits = options.word_count as f64 * wordlist_size.log2();

    // Adding a number adds ~3.3 bits (10 options) + log2(word_count) for position
    if options.include_number {
        #[allow(clippy::cast_precision_loss)]
        let position_bits = (options.word_count as f64).log2();
        entropy_bits += 10_f64.log2() + position_bits;
    }

    Ok(GeneratedPassphrase {
        passphrase,
        entropy_bits,
    })
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

        let result = generate_password_value(&options);
        assert!(
            result.is_ok(),
            "password generation should work: {result:?}"
        );
        let generated = result.unwrap_or_else(|_| GeneratedPassword {
            password: String::new(),
            entropy_bits: 0.0,
        });
        assert_eq!(generated.password.len(), 32);
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

        let result = generate_password_value(&options);
        assert!(
            result.is_ok(),
            "password generation should work: {result:?}"
        );
        let generated = result.unwrap_or_else(|_| GeneratedPassword {
            password: String::new(),
            entropy_bits: 0.0,
        });
        assert!(generated
            .password
            .chars()
            .all(|ch| !AMBIGUOUS_CHARS.contains(&ch)));
    }

    #[test]
    fn excludes_custom_characters() {
        let options = PasswordGeneratorOptions {
            length: 64,
            exclude_chars: Some("abcXYZ123".into()),
            ..Default::default()
        };

        let result = generate_password_value(&options);
        assert!(
            result.is_ok(),
            "password generation should work: {result:?}"
        );
        let generated = result.unwrap_or_else(|_| GeneratedPassword {
            password: String::new(),
            entropy_bits: 0.0,
        });
        assert!(generated
            .password
            .chars()
            .all(|ch| !['a', 'b', 'c', 'X', 'Y', 'Z', '1', '2', '3'].contains(&ch)));
    }

    #[test]
    fn generates_password_meeting_min_numbers() {
        let options = PasswordGeneratorOptions {
            length: 20,
            min_numbers: Some(5),
            ..Default::default()
        };

        let result = generate_password_value(&options);
        assert!(result.is_ok());
        let generated = result.unwrap_or_else(|_| GeneratedPassword {
            password: String::new(),
            entropy_bits: 0.0,
        });
        let digit_count = generated
            .password
            .chars()
            .filter(char::is_ascii_digit)
            .count();
        assert!(
            digit_count >= 5,
            "Expected at least 5 digits, got {digit_count} in '{}'",
            generated.password
        );
    }

    #[test]
    fn generates_password_meeting_min_symbols() {
        let options = PasswordGeneratorOptions {
            length: 20,
            min_symbols: Some(3),
            ..Default::default()
        };

        let result = generate_password_value(&options);
        assert!(result.is_ok());
        let generated = result.unwrap_or_else(|_| GeneratedPassword {
            password: String::new(),
            entropy_bits: 0.0,
        });
        let symbol_count = generated
            .password
            .chars()
            .filter(|c| SYMBOL_CHARS.contains(*c))
            .count();
        assert!(
            symbol_count >= 3,
            "Expected at least 3 symbols, got {symbol_count} in '{}'",
            generated.password
        );
    }

    #[test]
    fn rejects_min_numbers_when_numbers_disabled() {
        let options = PasswordGeneratorOptions {
            numbers: false,
            min_numbers: Some(3),
            ..Default::default()
        };

        let result = generate_password_value(&options);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_min_symbols_when_symbols_disabled() {
        let options = PasswordGeneratorOptions {
            symbols: false,
            min_symbols: Some(2),
            ..Default::default()
        };

        let result = generate_password_value(&options);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_min_exceeding_length() {
        let options = PasswordGeneratorOptions {
            length: 15,
            min_numbers: Some(10),
            min_symbols: Some(10),
            ..Default::default()
        };

        let result = generate_password_value(&options);
        assert!(result.is_err());
    }

    #[test]
    fn returns_entropy_bits() {
        let options = PasswordGeneratorOptions {
            length: 20,
            uppercase: true,
            lowercase: true,
            numbers: false,
            symbols: false,
            exclude_ambiguous: false,
            exclude_chars: None,
            min_numbers: None,
            min_symbols: None,
        };

        let result = generate_password_value(&options);
        assert!(result.is_ok());
        let generated = result.unwrap_or_else(|_| GeneratedPassword {
            password: String::new(),
            entropy_bits: 0.0,
        });

        // 52 chars (26 upper + 26 lower), entropy = 20 * log2(52)
        let expected = 20.0 * 52_f64.log2();
        assert!(
            (generated.entropy_bits - expected).abs() < 0.01,
            "Expected entropy ~{expected:.2}, got {:.2}",
            generated.entropy_bits
        );
    }

    #[test]
    fn generates_passphrase_with_correct_word_count() {
        let options = PassphraseGeneratorOptions {
            word_count: 6,
            include_number: false,
            ..Default::default()
        };

        let result = generate_passphrase_value(&options);
        assert!(result.is_ok());
        let generated = result.unwrap_or_else(|_| GeneratedPassphrase {
            passphrase: String::new(),
            entropy_bits: 0.0,
        });
        let word_count = generated.passphrase.split('-').count();
        assert_eq!(word_count, 6);
    }

    #[test]
    fn generates_passphrase_with_capitalization() {
        let options = PassphraseGeneratorOptions {
            word_count: 4,
            capitalize: true,
            include_number: false,
            ..Default::default()
        };

        let result = generate_passphrase_value(&options);
        assert!(result.is_ok());
        let generated = result.unwrap_or_else(|_| GeneratedPassphrase {
            passphrase: String::new(),
            entropy_bits: 0.0,
        });
        for word in generated.passphrase.split('-') {
            let first = word.chars().next();
            assert!(
                first.is_some_and(char::is_uppercase),
                "Expected uppercase first char in word '{word}'"
            );
        }
    }

    #[test]
    fn generates_passphrase_with_number() {
        let options = PassphraseGeneratorOptions {
            word_count: 4,
            include_number: true,
            ..Default::default()
        };

        let result = generate_passphrase_value(&options);
        assert!(result.is_ok());
        let generated = result.unwrap_or_else(|_| GeneratedPassphrase {
            passphrase: String::new(),
            entropy_bits: 0.0,
        });
        assert!(
            generated.passphrase.chars().any(|c| c.is_ascii_digit()),
            "Expected at least one digit in '{}'",
            generated.passphrase
        );
    }

    #[test]
    fn generates_passphrase_defaults() {
        let options = PassphraseGeneratorOptions::default();
        let result = generate_passphrase_value(&options);
        assert!(result.is_ok());
        let generated = result.unwrap_or_else(|_| GeneratedPassphrase {
            passphrase: String::new(),
            entropy_bits: 0.0,
        });
        assert!(!generated.passphrase.is_empty());
        assert!(generated.entropy_bits > 0.0);
    }
}
